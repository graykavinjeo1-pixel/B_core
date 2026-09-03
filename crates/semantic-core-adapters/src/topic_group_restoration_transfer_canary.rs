//! Frozen R39-RUN-0002 held-out topic/group restoration transfer suite.
//!
//! This oracle is frozen before the R39 product mechanism exists and is not
//! executed until the public diagnostic passes. It stresses cross-language
//! restoration, indexed history, revision refresh, long suspension, and
//! fail-closed authority boundaries with surfaces absent from RUN-0001.

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

#[derive(Clone, Copy)]
enum GroupChoice {
    First,
    Second,
    Composite,
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
        max_plan_steps: 16,
    }
}

fn run(
    api: &mut CognitiveApi,
    id: &str,
    turn: &mut u64,
    step: Step<'_>,
) -> semantic_core_adapters::ConversationTurnResponseIR {
    let response = api
        .process_conversation_turn(&request(id, *turn, step))
        .expect("R39 transfer turn");
    *turn += 1;
    response
}

fn value(response: &semantic_core_adapters::ConversationTurnResponseIR) -> Value {
    serde_json::to_value(response).expect("response json")
}

fn safe(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response.grounded_realization.validate()
        && response.grounded_realization.realized_text == response.output.text
        && response.grounded_realization.unsupported_claims == 0
        && !response.grounded_realization.semantic_authority
        && !response.grounded_realization.external_action_executed
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn groups(observed: &Value) -> &[Value] {
    observed
        .pointer("/conversation_state/active_discourse_groups")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn group_members(group: &Value) -> Vec<String> {
    sorted(
        group["member_keys"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
    )
}

fn select_group_id(observed: &Value, kind: &str, choice: GroupChoice) -> String {
    let mut candidates = groups(observed)
        .iter()
        .filter(|group| group["kind"] == kind)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left["introduced_turn"]
            .as_u64()
            .cmp(&right["introduced_turn"].as_u64())
            .then_with(|| left["group_id"].as_str().cmp(&right["group_id"].as_str()))
    });
    let selected = match choice {
        GroupChoice::First => candidates.first().copied(),
        GroupChoice::Second => candidates.get(1).copied(),
        GroupChoice::Composite => candidates.iter().copied().find(|group| {
            group["component_group_ids"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        }),
    };
    selected
        .and_then(|group| group["group_id"].as_str())
        .unwrap_or_default()
        .to_string()
}

fn group_by_id<'a>(observed: &'a Value, group_id: &str) -> Option<&'a Value> {
    groups(observed)
        .iter()
        .find(|group| group["group_id"] == group_id)
}

fn active_topic(observed: &Value) -> Option<&Value> {
    observed
        .pointer("/conversation_state/active_topics/0")
        .filter(|topic| topic["explicitly_activated"] == true)
}

fn topic_anchors_group(observed: &Value, group: &Value) -> bool {
    let Some(topic) = active_topic(observed) else {
        return false;
    };
    let kind = match group["kind"].as_str() {
        Some("ACTION") => "ACTION_GROUP",
        Some("ATTRIBUTED_PROPOSITION") => "ATTRIBUTED_PROPOSITION_GROUP",
        _ => return false,
    };
    topic["anchor_kind"] == kind
        && topic["anchor_group_id"] == group["group_id"]
        && topic["anchor_group_revision"] == group["revision"]
        && topic["anchor_membership_sha256"] == group["membership_sha256"]
        && topic["topic_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && topic["semantic_authority"] == false
        && topic["external_execution_authorized"] == false
}

fn grounded_transition(observed: &Value, group: &Value) -> bool {
    let Some(transition) = observed.pointer("/topic_transition") else {
        return false;
    };
    let Some(topic) = active_topic(observed) else {
        return false;
    };
    transition["schema"] == "B_CORE_TOPIC_TRANSITION_IR_1"
        && transition["kind"] == "RETURN_PREVIOUS"
        && transition["applied"] == true
        && transition["anchor_group_id"] == group["group_id"]
        && transition["anchor_group_revision"] == group["revision"]
        && transition["anchor_membership_sha256"] == group["membership_sha256"]
        && transition["transition_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && transition["semantic_authority"] == false
        && transition["external_action_executed"] == false
        && observed
            .pointer("/grounded_realization/claims")
            .and_then(Value::as_array)
            .is_some_and(|claims| {
                claims.iter().any(|claim| {
                    claim["kind"] == "DISCOURSE_TOPIC_TRANSITION"
                        && claim["evidence_refs"].as_array().is_some_and(|refs| {
                            refs.iter().any(|item| item == &topic["topic_sha256"])
                                && refs.iter().any(|item| item == &group["membership_sha256"])
                        })
                        && claim["semantic_authority"] == false
                        && claim["external_action_executed"] == false
                })
            })
}

fn targets(observed: &Value) -> Vec<String> {
    sorted(
        observed
            .pointer("/action_state_analysis/target_action_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
    )
}

fn binding_size(observed: &Value, kind: &str, size: usize) -> bool {
    observed
        .pointer("/reference_resolution/discourse_bindings")
        .and_then(Value::as_array)
        .is_some_and(|bindings| {
            bindings.iter().any(|binding| {
                binding["kind"] == kind
                    && binding["referent_ids"]
                        .as_array()
                        .is_some_and(|ids| ids.len() == size)
            })
        })
}

#[allow(clippy::too_many_arguments)]
fn action_case(
    id: &str,
    category: &str,
    setup: &[Step<'_>],
    choice: GroupChoice,
    activate: Step<'_>,
    suspended: &[Step<'_>],
    restore: Step<'_>,
    query: Step<'_>,
    expected_revision: u64,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut turn = 1;
    let mut latest = Value::Null;
    for step in setup {
        latest = value(&run(&mut api, id, &mut turn, *step));
    }
    let group_id = select_group_id(&latest, "ACTION", choice);
    run(&mut api, id, &mut turn, activate);
    for step in suspended {
        run(&mut api, id, &mut turn, *step);
    }
    let restored = run(&mut api, id, &mut turn, restore);
    let restored_value = value(&restored);
    let group = group_by_id(&restored_value, &group_id);
    let expected = group.map(group_members).unwrap_or_default();
    let queried = run(&mut api, id, &mut turn, query);
    let query_value = value(&queried);
    Row {
        id: id.to_string(),
        category: category.to_string(),
        pass: !group_id.is_empty()
            && group.is_some_and(|group| {
                group["revision"] == expected_revision
                    && topic_anchors_group(&restored_value, group)
                    && grounded_transition(&restored_value, group)
            })
            && !expected.is_empty()
            && targets(&query_value) == expected
            && binding_size(&query_value, "PLURAL_EVENT_REFERENCE", expected.len())
            && safe(&restored)
            && safe(&queried),
        trace: vec![restored_value.to_string(), query_value.to_string()],
    }
}

#[allow(clippy::too_many_arguments)]
fn speaker_case(
    id: &str,
    category: &str,
    setup: &[Step<'_>],
    choice: GroupChoice,
    activate: Step<'_>,
    suspended: &[Step<'_>],
    restore: Step<'_>,
    query: Step<'_>,
    expected_revision: u64,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut turn = 1;
    let mut latest = Value::Null;
    for step in setup {
        latest = value(&run(&mut api, id, &mut turn, *step));
    }
    let group_id = select_group_id(&latest, "ATTRIBUTED_PROPOSITION", choice);
    run(&mut api, id, &mut turn, activate);
    for step in suspended {
        run(&mut api, id, &mut turn, *step);
    }
    let restored = run(&mut api, id, &mut turn, restore);
    let restored_value = value(&restored);
    let group = group_by_id(&restored_value, &group_id);
    let expected_size = group.map(group_members).map_or(0, |items| items.len());
    let queried = run(&mut api, id, &mut turn, query);
    let query_value = value(&queried);
    Row {
        id: id.to_string(),
        category: category.to_string(),
        pass: !group_id.is_empty()
            && group.is_some_and(|group| {
                group["revision"] == expected_revision
                    && topic_anchors_group(&restored_value, group)
                    && grounded_transition(&restored_value, group)
            })
            && expected_size > 0
            && binding_size(&query_value, "PLURAL_PROPOSITION_REFERENCE", expected_size)
            && safe(&restored)
            && safe(&queried),
        trace: vec![restored_value.to_string(), query_value.to_string()],
    }
}

fn invalid_case(id: &str, setup: &[Step<'_>], request_step: Step<'_>, quoted: bool) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut turn = 1;
    let mut latest = Value::Null;
    for step in setup {
        latest = value(&run(&mut api, id, &mut turn, *step));
    }
    let groups_before = latest
        .pointer("/conversation_state/active_discourse_groups")
        .cloned();
    let ledger_before = latest
        .pointer("/conversation_state/action_state_ledger")
        .cloned();
    let topics_before = latest.pointer("/conversation_state/active_topics").cloned();
    let response = run(&mut api, id, &mut turn, request_step);
    let observed = value(&response);
    let transition = observed.pointer("/topic_transition");
    let boundary = if quoted {
        transition.is_none()
            && response.disposition != ConversationTurnDispositionIR::ClarificationRequired
    } else {
        transition.is_some_and(|item| {
            item["kind"] == "UNRESOLVED"
                && item["applied"] == false
                && item["unresolved_terms"]
                    .as_array()
                    .is_some_and(|terms| !terms.is_empty())
                && item["semantic_authority"] == false
                && item["external_action_executed"] == false
                && item["transition_sha256"]
                    .as_str()
                    .is_some_and(|hash| hash.len() == 64)
        }) && response.disposition == ConversationTurnDispositionIR::ClarificationRequired
    };
    Row {
        id: id.to_string(),
        category: "safety_and_authority_boundary".to_string(),
        pass: boundary
            && groups_before
                == observed
                    .pointer("/conversation_state/active_discourse_groups")
                    .cloned()
            && ledger_before
                == observed
                    .pointer("/conversation_state/action_state_ledger")
                    .cloned()
            && topics_before
                == observed
                    .pointer("/conversation_state/active_topics")
                    .cloned()
            && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn neutral_steps(language: LanguageCodeIR) -> Vec<Step<'static>> {
    let text = match language {
        LanguageCodeIR::Korean => "응, 알겠어",
        _ => "Okay, noted.",
    };
    (0..18).map(|_| Step { text, language }).collect()
}

#[allow(clippy::vec_init_then_push)]
fn main() {
    let en = LanguageCodeIR::English;
    let ko = LanguageCodeIR::Korean;
    let mut rows = Vec::new();

    rows.push(action_case(
        "R39_XFER_XL_01",
        "cross_language_group_restore",
        &[Step {
            text: "check parser and repair index",
            language: en,
        }],
        GroupChoice::First,
        Step {
            text: "그 작업 묶음을 대화 주제로 기억해 둬",
            language: ko,
        },
        &[Step {
            text: "Switch to the deployment topic.",
            language: en,
        }],
        Step {
            text: "직전 주제로 복귀하자",
            language: ko,
        },
        Step {
            text: "Give me that task group's progress.",
            language: en,
        },
        1,
    ));
    rows.push(action_case(
        "R39_XFER_XL_02",
        "cross_language_group_restore",
        &[
            Step {
                text: "파서를 분석하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "배포기를 확인하고 릴리스를 수리해",
                language: ko,
            },
        ],
        GroupChoice::Second,
        Step {
            text: "Pin the second task group as the discussion topic.",
            language: en,
        },
        &[Step {
            text: "배포 주제로 전환하자",
            language: ko,
        }],
        Step {
            text: "Resume the prior topic.",
            language: en,
        },
        Step {
            text: "그 작업 묶음 진행 상황은 어때?",
            language: ko,
        },
        1,
    ));
    rows.push(speaker_case(
        "R39_XFER_XL_03",
        "cross_language_group_restore",
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
                text: "What did Dana and Eli report?",
                language: en,
            },
        ],
        GroupChoice::First,
        Step {
            text: "그 화자 묶음을 대화 주제로 기억해 둬",
            language: ko,
        },
        &[Step {
            text: "Switch to the release topic.",
            language: en,
        }],
        Step {
            text: "직전 주제로 복귀하자",
            language: ko,
        },
        Step {
            text: "What did that speaker group report?",
            language: en,
        },
        1,
    ));
    rows.push(speaker_case(
        "R39_XFER_XL_04",
        "cross_language_group_restore",
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
                text: "다나와 엘리는 뭐라고 보고했어?",
                language: ko,
            },
        ],
        GroupChoice::First,
        Step {
            text: "Pin that speaker group as the discussion topic.",
            language: en,
        },
        &[Step {
            text: "릴리스 주제로 전환하자",
            language: ko,
        }],
        Step {
            text: "Resume the prior topic.",
            language: en,
        },
        Step {
            text: "그 화자 묶음은 뭐라고 보고했어?",
            language: ko,
        },
        1,
    ));

    rows.push(action_case(
        "R39_XFER_IDX_01",
        "indexed_topic_history_restore",
        &[Step {
            text: "inspect parser and repair index",
            language: en,
        }],
        GroupChoice::First,
        Step {
            text: "Pin that task group as the discussion topic.",
            language: en,
        },
        &[
            Step {
                text: "Switch to the release topic.",
                language: en,
            },
            Step {
                text: "Switch to the deployment topic.",
                language: en,
            },
        ],
        Step {
            text: "Return to the topic from two topics ago.",
            language: en,
        },
        Step {
            text: "Give me that task group's progress.",
            language: en,
        },
        1,
    ));
    rows.push(action_case(
        "R39_XFER_IDX_02",
        "indexed_topic_history_restore",
        &[
            Step {
                text: "파서를 확인하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "배포기를 분석하고 릴리스를 수리해",
                language: ko,
            },
            Step {
                text: "첫 작업 묶음과 둘째 작업 묶음을 결합해",
                language: ko,
            },
        ],
        GroupChoice::Composite,
        Step {
            text: "결합된 작업 묶음을 대화 주제로 기억해 둬",
            language: ko,
        },
        &[
            Step {
                text: "릴리스 주제로 전환하자",
                language: ko,
            },
            Step {
                text: "배포 주제로 전환하자",
                language: ko,
            },
        ],
        Step {
            text: "두 주제 전으로 복귀하자",
            language: ko,
        },
        Step {
            text: "그 작업 묶음 진행 상황은 어때?",
            language: ko,
        },
        1,
    ));
    rows.push(speaker_case(
        "R39_XFER_IDX_03",
        "indexed_topic_history_restore",
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
                text: "What did Dana and Eli report?",
                language: en,
            },
        ],
        GroupChoice::First,
        Step {
            text: "Pin that speaker group as the discussion topic.",
            language: en,
        },
        &[
            Step {
                text: "Switch to the release topic.",
                language: en,
            },
            Step {
                text: "Switch to the deployment topic.",
                language: en,
            },
        ],
        Step {
            text: "Return to the topic from two topics ago.",
            language: en,
        },
        Step {
            text: "What did that speaker group report?",
            language: en,
        },
        1,
    ));
    rows.push(speaker_case(
        "R39_XFER_IDX_04",
        "indexed_topic_history_restore",
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
                text: "다나와 엘리는 뭐라고 보고했어?",
                language: ko,
            },
        ],
        GroupChoice::First,
        Step {
            text: "그 화자 묶음을 대화 주제로 기억해 둬",
            language: ko,
        },
        &[
            Step {
                text: "릴리스 주제로 전환하자",
                language: ko,
            },
            Step {
                text: "배포 주제로 전환하자",
                language: ko,
            },
        ],
        Step {
            text: "두 주제 전으로 복귀하자",
            language: ko,
        },
        Step {
            text: "그 화자 묶음은 뭐라고 보고했어?",
            language: ko,
        },
        1,
    ));

    rows.push(action_case(
        "R39_XFER_REV_01",
        "reversible_revision_refresh",
        &[
            Step {
                text: "inspect parser and repair index",
                language: en,
            },
            Step {
                text: "analyze deployer",
                language: en,
            },
        ],
        GroupChoice::First,
        Step {
            text: "Pin the first task group as the discussion topic.",
            language: en,
        },
        &[
            Step {
                text: "Switch to the release topic.",
                language: en,
            },
            Step {
                text: "Attach deployer to the first task group.",
                language: en,
            },
            Step {
                text: "Detach deployer from the first task group.",
                language: en,
            },
        ],
        Step {
            text: "Resume the prior topic.",
            language: en,
        },
        Step {
            text: "Give me that task group's progress.",
            language: en,
        },
        3,
    ));
    rows.push(action_case(
        "R39_XFER_REV_02",
        "reversible_revision_refresh",
        &[
            Step {
                text: "파서를 확인하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "배포기를 분석해",
                language: ko,
            },
        ],
        GroupChoice::First,
        Step {
            text: "첫 작업 묶음을 대화 주제로 기억해 둬",
            language: ko,
        },
        &[
            Step {
                text: "릴리스 주제로 전환하자",
                language: ko,
            },
            Step {
                text: "첫 작업 묶음에 배포기를 추가해",
                language: ko,
            },
            Step {
                text: "첫 작업 묶음에서 배포기를 제거해",
                language: ko,
            },
        ],
        Step {
            text: "직전 주제로 복귀하자",
            language: ko,
        },
        Step {
            text: "그 작업 묶음 진행 상황은 어때?",
            language: ko,
        },
        3,
    ));
    rows.push(speaker_case(
        "R39_XFER_REV_03",
        "reversible_revision_refresh",
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
                text: "What did Dana and Eli report?",
                language: en,
            },
            Step {
                text: "Faye says the deployer is ready.",
                language: en,
            },
        ],
        GroupChoice::First,
        Step {
            text: "Pin the first speaker group as the discussion topic.",
            language: en,
        },
        &[
            Step {
                text: "Switch to the release topic.",
                language: en,
            },
            Step {
                text: "Attach Faye to the first speaker group.",
                language: en,
            },
            Step {
                text: "Detach Faye from the first speaker group.",
                language: en,
            },
        ],
        Step {
            text: "Resume the prior topic.",
            language: en,
        },
        Step {
            text: "What did that speaker group report?",
            language: en,
        },
        3,
    ));
    rows.push(speaker_case(
        "R39_XFER_REV_04",
        "reversible_revision_refresh",
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
                text: "다나와 엘리는 뭐라고 보고했어?",
                language: ko,
            },
            Step {
                text: "페이는 배포기가 준비됐다고 말했다.",
                language: ko,
            },
        ],
        GroupChoice::First,
        Step {
            text: "첫 화자 묶음을 대화 주제로 기억해 둬",
            language: ko,
        },
        &[
            Step {
                text: "릴리스 주제로 전환하자",
                language: ko,
            },
            Step {
                text: "첫 화자 묶음에 페이를 추가해",
                language: ko,
            },
            Step {
                text: "첫 화자 묶음에서 페이를 제거해",
                language: ko,
            },
        ],
        Step {
            text: "직전 주제로 복귀하자",
            language: ko,
        },
        Step {
            text: "그 화자 묶음은 뭐라고 보고했어?",
            language: ko,
        },
        3,
    ));

    let mut long_en = vec![Step {
        text: "Switch to the release topic.",
        language: en,
    }];
    long_en.extend(neutral_steps(en));
    let mut long_ko = vec![Step {
        text: "릴리스 주제로 전환하자",
        language: ko,
    }];
    long_ko.extend(neutral_steps(ko));
    rows.push(action_case(
        "R39_XFER_LONG_01",
        "long_horizon_group_restore",
        &[Step {
            text: "inspect parser and repair index",
            language: en,
        }],
        GroupChoice::First,
        Step {
            text: "Pin that task group as the discussion topic.",
            language: en,
        },
        &long_en,
        Step {
            text: "Resume the prior topic.",
            language: en,
        },
        Step {
            text: "Give me that task group's progress.",
            language: en,
        },
        1,
    ));
    rows.push(action_case(
        "R39_XFER_LONG_02",
        "long_horizon_group_restore",
        &[Step {
            text: "파서를 확인하고 인덱스를 수리해",
            language: ko,
        }],
        GroupChoice::First,
        Step {
            text: "그 작업 묶음을 대화 주제로 기억해 둬",
            language: ko,
        },
        &long_ko,
        Step {
            text: "직전 주제로 복귀하자",
            language: ko,
        },
        Step {
            text: "그 작업 묶음 진행 상황은 어때?",
            language: ko,
        },
        1,
    ));
    rows.push(speaker_case(
        "R39_XFER_LONG_03",
        "long_horizon_group_restore",
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
                text: "What did Dana and Eli report?",
                language: en,
            },
        ],
        GroupChoice::First,
        Step {
            text: "Pin that speaker group as the discussion topic.",
            language: en,
        },
        &long_en,
        Step {
            text: "Resume the prior topic.",
            language: en,
        },
        Step {
            text: "What did that speaker group report?",
            language: en,
        },
        1,
    ));
    rows.push(speaker_case(
        "R39_XFER_LONG_04",
        "long_horizon_group_restore",
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
                text: "다나와 엘리는 뭐라고 보고했어?",
                language: ko,
            },
        ],
        GroupChoice::First,
        Step {
            text: "그 화자 묶음을 대화 주제로 기억해 둬",
            language: ko,
        },
        &long_ko,
        Step {
            text: "직전 주제로 복귀하자",
            language: ko,
        },
        Step {
            text: "그 화자 묶음은 뭐라고 보고했어?",
            language: ko,
        },
        1,
    ));

    rows.push(invalid_case(
        "R39_XFER_SAFE_01",
        &[Step {
            text: "inspect parser and repair index",
            language: en,
        }],
        Step {
            text: "Explain the sentence ‘Pin that task group as the discussion topic.’",
            language: en,
        },
        true,
    ));
    rows.push(invalid_case(
        "R39_XFER_SAFE_02",
        &[
            Step {
                text: "파서를 확인하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "배포기를 분석하고 릴리스를 수리해",
                language: ko,
            },
        ],
        Step {
            text: "그 작업 묶음을 대화 주제로 기억해 둬",
            language: ko,
        },
        false,
    ));
    rows.push(invalid_case(
        "R39_XFER_SAFE_03",
        &[Step {
            text: "inspect parser and repair index",
            language: en,
        }],
        Step {
            text: "Pin that speaker group as the discussion topic.",
            language: en,
        },
        false,
    ));
    rows.push(invalid_case(
        "R39_XFER_SAFE_04",
        &[
            Step {
                text: "파서를 확인하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "그 작업 묶음을 대화 주제로 기억해 둬",
                language: ko,
            },
            Step {
                text: "릴리스 주제로 전환하자",
                language: ko,
            },
        ],
        Step {
            text: "다섯 주제 전으로 복귀하자",
            language: ko,
        },
        false,
    ));

    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "R39-RUN-0002",
            "frozen_before_product_changes": true,
            "held_out_until_diagnostic_passes": true,
            "total": rows.len(),
            "passed": passed,
            "failed": rows.len() - passed,
            "external_llm_calls": 0,
            "local_teacher_calls": 0,
            "network_calls": 0,
            "recursive_source_mutations": 0,
            "rows": rows,
        }))
        .expect("suite json")
    );
}
