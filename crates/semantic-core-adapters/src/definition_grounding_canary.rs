//! Frozen R44 diagnostic suite for conversational definition grounding.
//!
//! The cases use only the public conversation API. They require an explicit,
//! non-authoritative definition record before a fresh label may enter the
//! existing compositional GoalIR path.

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
enum Check {
    Bound {
        turn: usize,
        alias: &'static str,
        canonical: &'static str,
    },
    Plan {
        turn: usize,
        intent: &'static str,
        canonical: &'static str,
    },
    NoPlan {
        turn: usize,
    },
    SharedPayload {
        left: usize,
        right: usize,
    },
    Conflict {
        turn: usize,
    },
    Rejected {
        turn: usize,
    },
    Blocked {
        turn: usize,
        canonical: &'static str,
    },
    Quoted {
        turn: usize,
        canonical: &'static str,
    },
    Subject {
        turn: usize,
        surface: &'static str,
    },
}

struct Case {
    id: &'static str,
    category: &'static str,
    turns: Vec<(&'static str, LanguageCodeIR)>,
    checks: Vec<Check>,
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

fn response_contract(value: &Value) -> bool {
    value["schema"] == CONVERSATION_TURN_RESPONSE_SCHEMA
        && value
            .get("definition_grounding")
            .is_some_and(|item| !item.is_null())
        && value.pointer("/six_axis_integration/complete") == Some(&Value::Bool(true))
        && value.pointer("/six_axis_integration/semantic_authority") == Some(&Value::Bool(false))
        && value.pointer("/six_axis_integration/language_can_execute") == Some(&Value::Bool(false))
        && value.pointer("/six_axis_integration/package_boundary/valid") == Some(&Value::Bool(true))
        && value.pointer("/grounded_realization/unsupported_claims")
            == Some(&Value::Number(0_u64.into()))
}

fn has_verified_result(value: &Value) -> bool {
    value
        .pointer("/interaction_provenance/nodes")
        .and_then(Value::as_array)
        .is_some_and(|nodes| nodes.iter().any(|node| node["kind"] == "VERIFIED_RESULT"))
}

fn check(values: &[Value], check: Check) -> bool {
    let at = |turn: usize| values.get(turn.saturating_sub(1));
    match check {
        Check::Bound {
            turn,
            alias,
            canonical,
        } => at(turn).is_some_and(|value| {
            value.pointer("/definition_grounding/disposition")
                == Some(&Value::String("BOUND".into()))
                && value
                    .pointer("/definition_grounding/binding/alias_surface")
                    .and_then(Value::as_str)
                    == Some(alias)
                && value
                    .pointer("/definition_grounding/binding/canonical_predicate")
                    .and_then(Value::as_str)
                    == Some(canonical)
                && value.pointer("/definition_grounding/binding/semantic_authority")
                    == Some(&Value::Bool(false))
                && value
                    .pointer("/definition_grounding/binding/external_action_execution_authorized")
                    == Some(&Value::Bool(false))
                && value
                    .pointer("/definition_grounding/binding/semantic_payload_sha256")
                    .and_then(Value::as_str)
                    .is_some_and(|hash| hash.len() == 64)
                && value.get("grounded_response").is_none_or(Value::is_null)
                && !has_verified_result(value)
        }),
        Check::Plan {
            turn,
            intent,
            canonical,
        } => at(turn).is_some_and(|value| {
            value
                .pointer("/grounded_response/plan/intent")
                .and_then(Value::as_str)
                == Some(intent)
                && value
                    .pointer("/pragmatic_interpretation/compositional_analysis/frames")
                    .and_then(Value::as_array)
                    .is_some_and(|frames| {
                        frames
                            .iter()
                            .any(|frame| frame["canonical_predicate"] == canonical)
                    })
                && !has_verified_result(value)
        }),
        Check::NoPlan { turn } => at(turn).is_some_and(|value| {
            value.get("grounded_response").is_none_or(Value::is_null) && !has_verified_result(value)
        }),
        Check::SharedPayload { left, right } => at(left).zip(at(right)).is_some_and(|(a, b)| {
            let a = a
                .pointer("/definition_grounding/binding/semantic_payload_sha256")
                .and_then(Value::as_str);
            let b = b
                .pointer("/definition_grounding/binding/semantic_payload_sha256")
                .and_then(Value::as_str);
            a.is_some() && a == b
        }),
        Check::Conflict { turn } => at(turn).is_some_and(|value| {
            value.pointer("/definition_grounding/disposition")
                == Some(&Value::String("CONFLICT_REJECTED".into()))
                && value
                    .pointer("/definition_grounding/binding")
                    .is_none_or(Value::is_null)
                && value.get("grounded_response").is_none_or(Value::is_null)
        }),
        Check::Rejected { turn } => at(turn).is_some_and(|value| {
            value
                .pointer("/definition_grounding/disposition")
                .and_then(Value::as_str)
                .is_some_and(|item| item.ends_with("_REJECTED"))
                && value
                    .pointer("/definition_grounding/binding")
                    .is_none_or(Value::is_null)
                && value.get("grounded_response").is_none_or(Value::is_null)
        }),
        Check::Blocked { turn, canonical } => at(turn).is_some_and(|value| {
            value
                .pointer("/pragmatic_interpretation/compositional_analysis/candidates")
                .and_then(Value::as_array)
                .is_some_and(|candidates| {
                    candidates.iter().any(|candidate| {
                        candidate["disposition"] == "BLOCKED_BY_NEGATION"
                            && value
                                .pointer("/pragmatic_interpretation/compositional_analysis/frames")
                                .and_then(Value::as_array)
                                .is_some_and(|frames| {
                                    frames.iter().any(|frame| {
                                        frame["frame_id"] == candidate["source_frame_id"]
                                            && frame["canonical_predicate"] == canonical
                                    })
                                })
                    })
                })
        }),
        Check::Quoted { turn, canonical } => at(turn).is_some_and(|value| {
            value
                .pointer("/pragmatic_interpretation/compositional_analysis/frames")
                .and_then(Value::as_array)
                .is_some_and(|frames| {
                    frames.iter().any(|frame| {
                        frame["canonical_predicate"] == canonical
                            && frame["embedded_under_quote"] == true
                            && frame["external_execution_authorized"] == false
                    })
                })
        }),
        Check::Subject { turn, surface } => at(turn).is_some_and(|value| {
            value
                .pointer("/grounded_response/understanding/subject")
                .and_then(Value::as_str)
                .is_some_and(|subject| subject.to_lowercase().contains(&surface.to_lowercase()))
        }),
    }
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut values = Vec::new();
    let mut trace = Vec::new();
    for (offset, (text, language)) in case.turns.iter().enumerate() {
        let response = api
            .process_conversation_turn(&request(
                case.id,
                u64::try_from(offset + 1).expect("bounded turn"),
                text,
                *language,
            ))
            .expect("turn");
        trace.push(response.output.text.clone());
        values.push(serde_json::to_value(response).expect("json"));
    }
    let pass = values.iter().all(response_contract)
        && case.checks.iter().copied().all(|item| check(&values, item));
    Row {
        id: case.id.into(),
        category: case.category.into(),
        pass,
        trace,
    }
}

fn cases() -> Vec<Case> {
    use Check::{Blocked, Bound, Conflict, NoPlan, Plan, Quoted, Rejected, SharedPayload, Subject};
    use LanguageCodeIR::{English as En, Korean as Ko};
    vec![
        Case {
            id: "R44_KO_01",
            category: "korean_opaque_definition",
            turns: vec![
                ("이 대화에서 \"무루\"는 검사하라는 뜻이야.", Ko),
                ("캐시를 무루해줘.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "무루",
                    canonical: "INVESTIGATE",
                },
                Plan {
                    turn: 2,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
            ],
        },
        Case {
            id: "R44_KO_02",
            category: "korean_opaque_definition",
            turns: vec![
                ("\"가람\"은 수리하라는 뜻이야.", Ko),
                ("파서를 가람해줘.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "가람",
                    canonical: "REPAIR",
                },
                Plan {
                    turn: 2,
                    intent: "REPAIR",
                    canonical: "REPAIR",
                },
            ],
        },
        Case {
            id: "R44_KO_03",
            category: "korean_opaque_definition",
            turns: vec![
                ("앞으로 \"새롬\"을 생성하라는 뜻으로 써.", Ko),
                ("보고서를 새롬해줘.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "새롬",
                    canonical: "CREATE",
                },
                Plan {
                    turn: 2,
                    intent: "CREATE",
                    canonical: "CREATE",
                },
            ],
        },
        Case {
            id: "R44_KO_04",
            category: "korean_opaque_definition",
            turns: vec![
                ("\"노을\"은 삭제하라는 뜻이야.", Ko),
                ("임시 파일을 노을해줘.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "노을",
                    canonical: "DELETE",
                },
                Plan {
                    turn: 2,
                    intent: "EXECUTE",
                    canonical: "DELETE",
                },
            ],
        },
        Case {
            id: "R44_EN_01",
            category: "english_opaque_definition",
            turns: vec![
                ("In this conversation, \"nexel\" means inspect.", En),
                ("Nexel the cache.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "nexel",
                    canonical: "INVESTIGATE",
                },
                Plan {
                    turn: 2,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
            ],
        },
        Case {
            id: "R44_EN_02",
            category: "english_opaque_definition",
            turns: vec![("\"vorda\" means repair.", En), ("Vorda the parser.", En)],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "vorda",
                    canonical: "REPAIR",
                },
                Plan {
                    turn: 2,
                    intent: "REPAIR",
                    canonical: "REPAIR",
                },
            ],
        },
        Case {
            id: "R44_EN_03",
            category: "english_opaque_definition",
            turns: vec![
                ("Use \"pelkin\" to mean create.", En),
                ("Pelkin the report.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "pelkin",
                    canonical: "CREATE",
                },
                Plan {
                    turn: 2,
                    intent: "CREATE",
                    canonical: "CREATE",
                },
            ],
        },
        Case {
            id: "R44_EN_04",
            category: "english_opaque_definition",
            turns: vec![
                ("From now on, \"drax\" means delete.", En),
                ("Drax the temporary file.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "drax",
                    canonical: "DELETE",
                },
                Plan {
                    turn: 2,
                    intent: "EXECUTE",
                    canonical: "DELETE",
                },
            ],
        },
        Case {
            id: "R44_SHARED_01",
            category: "shared_semantic_payload",
            turns: vec![
                ("\"무루\"는 검사하라는 뜻이야.", Ko),
                ("\"nexel\" means inspect.", En),
                ("Nexel the cache.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "무루",
                    canonical: "INVESTIGATE",
                },
                Bound {
                    turn: 2,
                    alias: "nexel",
                    canonical: "INVESTIGATE",
                },
                SharedPayload { left: 1, right: 2 },
                Plan {
                    turn: 3,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
            ],
        },
        Case {
            id: "R44_SHARED_02",
            category: "shared_semantic_payload",
            turns: vec![
                ("\"가람\"은 수리하라는 뜻이야.", Ko),
                ("\"vorda\" means repair.", En),
                ("큐를 가람해줘.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "가람",
                    canonical: "REPAIR",
                },
                Bound {
                    turn: 2,
                    alias: "vorda",
                    canonical: "REPAIR",
                },
                SharedPayload { left: 1, right: 2 },
                Plan {
                    turn: 3,
                    intent: "REPAIR",
                    canonical: "REPAIR",
                },
            ],
        },
        Case {
            id: "R44_CHAIN_01",
            category: "alias_chain",
            turns: vec![
                ("\"nexel\" means inspect.", En),
                ("\"sora\" means nexel.", En),
                ("Sora the worker.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "nexel",
                    canonical: "INVESTIGATE",
                },
                Bound {
                    turn: 2,
                    alias: "sora",
                    canonical: "INVESTIGATE",
                },
                SharedPayload { left: 1, right: 2 },
                Plan {
                    turn: 3,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
            ],
        },
        Case {
            id: "R44_CHAIN_02",
            category: "alias_chain",
            turns: vec![
                ("\"가람\"은 수리하라는 뜻이야.", Ko),
                ("\"보라\"는 가람하라는 뜻이야.", Ko),
                ("서버를 보라해줘.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "가람",
                    canonical: "REPAIR",
                },
                Bound {
                    turn: 2,
                    alias: "보라",
                    canonical: "REPAIR",
                },
                SharedPayload { left: 1, right: 2 },
                Plan {
                    turn: 3,
                    intent: "REPAIR",
                    canonical: "REPAIR",
                },
            ],
        },
        Case {
            id: "R44_SCOPE_01",
            category: "learned_scope",
            turns: vec![
                ("\"가람\"은 수리하라는 뜻이야.", Ko),
                ("캐시를 가람하지 말고 원인을 설명해줘.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "가람",
                    canonical: "REPAIR",
                },
                Plan {
                    turn: 2,
                    intent: "EXPLAIN",
                    canonical: "EXPLAIN",
                },
                Blocked {
                    turn: 2,
                    canonical: "REPAIR",
                },
            ],
        },
        Case {
            id: "R44_SCOPE_02",
            category: "learned_scope",
            turns: vec![
                ("\"vorda\" means repair.", En),
                ("Do not vorda the cache; explain the cause.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "vorda",
                    canonical: "REPAIR",
                },
                Plan {
                    turn: 2,
                    intent: "EXPLAIN",
                    canonical: "EXPLAIN",
                },
                Blocked {
                    turn: 2,
                    canonical: "REPAIR",
                },
            ],
        },
        Case {
            id: "R44_SCOPE_03",
            category: "learned_scope",
            turns: vec![
                ("\"노을\"은 삭제하라는 뜻이야.", Ko),
                ("'캐시를 노을해'라는 문장을 설명해줘.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "노을",
                    canonical: "DELETE",
                },
                Plan {
                    turn: 2,
                    intent: "EXPLAIN",
                    canonical: "EXPLAIN",
                },
                Quoted {
                    turn: 2,
                    canonical: "DELETE",
                },
            ],
        },
        Case {
            id: "R44_SCOPE_04",
            category: "learned_scope",
            turns: vec![
                ("\"drax\" means delete.", En),
                ("Explain the phrase 'drax the cache'.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "drax",
                    canonical: "DELETE",
                },
                Plan {
                    turn: 2,
                    intent: "EXPLAIN",
                    canonical: "EXPLAIN",
                },
                Quoted {
                    turn: 2,
                    canonical: "DELETE",
                },
            ],
        },
        Case {
            id: "R44_DELAY_01",
            category: "definition_persistence",
            turns: vec![
                ("\"무루\"는 검사하라는 뜻이야.", Ko),
                ("음...", Ko),
                ("고마워", Ko),
                ("잠깐", Ko),
                ("알겠어", Ko),
                ("캐시를 무루해줘.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "무루",
                    canonical: "INVESTIGATE",
                },
                Plan {
                    turn: 6,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
            ],
        },
        Case {
            id: "R44_DELAY_02",
            category: "definition_persistence",
            turns: vec![
                ("\"nexel\" means inspect.", En),
                ("uh...", En),
                ("thanks", En),
                ("one moment", En),
                ("okay", En),
                ("Nexel the queue.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "nexel",
                    canonical: "INVESTIGATE",
                },
                Plan {
                    turn: 6,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
            ],
        },
        Case {
            id: "R44_DELAY_03",
            category: "definition_persistence",
            turns: vec![
                ("In this conversation, \"siven\" means repair.", En),
                ("음...", Ko),
                ("고마워", Ko),
                ("잠깐", Ko),
                ("큐를 siven해줘.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "siven",
                    canonical: "REPAIR",
                },
                Plan {
                    turn: 5,
                    intent: "REPAIR",
                    canonical: "REPAIR",
                },
            ],
        },
        Case {
            id: "R44_DELAY_04",
            category: "definition_persistence",
            turns: vec![
                ("이 대화에서 \"모라\"는 검사하라는 뜻이야.", Ko),
                ("uh...", En),
                ("thanks", En),
                ("one moment", En),
                ("로그를 모라해줘.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "모라",
                    canonical: "INVESTIGATE",
                },
                Plan {
                    turn: 5,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
            ],
        },
        Case {
            id: "R44_REJECT_01",
            category: "definition_conflict",
            turns: vec![
                ("\"nexel\" means inspect.", En),
                ("\"nexel\" means delete.", En),
                ("Nexel the cache.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "nexel",
                    canonical: "INVESTIGATE",
                },
                Conflict { turn: 2 },
                Plan {
                    turn: 3,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
            ],
        },
        Case {
            id: "R44_REJECT_02",
            category: "definition_rejection",
            turns: vec![
                ("Alice said 'zorv means delete'.", En),
                ("Zorv the cache.", En),
            ],
            checks: vec![Rejected { turn: 1 }, NoPlan { turn: 2 }],
        },
        Case {
            id: "R44_REJECT_03",
            category: "definition_rejection",
            turns: vec![
                ("If \"zorv\" meant delete, would that help?", En),
                ("Zorv the cache.", En),
            ],
            checks: vec![Rejected { turn: 1 }, NoPlan { turn: 2 }],
        },
        Case {
            id: "R44_REJECT_04",
            category: "definition_rejection",
            turns: vec![
                ("\"zorv\" means inspect or delete.", En),
                ("Zorv the cache.", En),
            ],
            checks: vec![Rejected { turn: 1 }, NoPlan { turn: 2 }],
        },
        Case {
            id: "R44_ELLIPSIS_01",
            category: "learned_discourse_reuse",
            turns: vec![
                ("\"무루\"는 검사하라는 뜻이야.", Ko),
                ("캐시를 무루해줘.", Ko),
                ("그거 다시 해.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "무루",
                    canonical: "INVESTIGATE",
                },
                Plan {
                    turn: 2,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
                Plan {
                    turn: 3,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
                Subject {
                    turn: 3,
                    surface: "캐시",
                },
            ],
        },
        Case {
            id: "R44_ELLIPSIS_02",
            category: "learned_discourse_reuse",
            turns: vec![
                ("\"vorda\" means repair.", En),
                ("Vorda the parser.", En),
                ("Do that again.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "vorda",
                    canonical: "REPAIR",
                },
                Plan {
                    turn: 2,
                    intent: "REPAIR",
                    canonical: "REPAIR",
                },
                Plan {
                    turn: 3,
                    intent: "REPAIR",
                    canonical: "REPAIR",
                },
                Subject {
                    turn: 3,
                    surface: "parser",
                },
            ],
        },
        Case {
            id: "R44_ELLIPSIS_03",
            category: "learned_discourse_reuse",
            turns: vec![
                ("\"nexel\" means inspect.", En),
                ("Nexel the cache.", En),
                ("서버를 검사해.", Ko),
                ("캐시 얘기로 돌아가자.", Ko),
                ("Do it again.", En),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "nexel",
                    canonical: "INVESTIGATE",
                },
                Plan {
                    turn: 5,
                    intent: "INVESTIGATE",
                    canonical: "INVESTIGATE",
                },
                Subject {
                    turn: 5,
                    surface: "cache",
                },
            ],
        },
        Case {
            id: "R44_ELLIPSIS_04",
            category: "learned_discourse_reuse",
            turns: vec![
                ("\"가람\"은 수리하라는 뜻이야.", Ko),
                ("파서를 가람해줘.", Ko),
                ("Repair the worker.", En),
                ("파서 얘기로 돌아가자.", Ko),
                ("그거 다시 해.", Ko),
            ],
            checks: vec![
                Bound {
                    turn: 1,
                    alias: "가람",
                    canonical: "REPAIR",
                },
                Plan {
                    turn: 5,
                    intent: "REPAIR",
                    canonical: "REPAIR",
                },
                Subject {
                    turn: 5,
                    surface: "파서",
                },
            ],
        },
    ]
}

fn main() {
    let rows = cases().into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    println!("{}", serde_json::to_string_pretty(&rows).expect("rows"));
    println!("R44_DIAGNOSTIC_PASSED={passed}/{}", rows.len());
    if passed != rows.len() {
        std::process::exit(1);
    }
}
