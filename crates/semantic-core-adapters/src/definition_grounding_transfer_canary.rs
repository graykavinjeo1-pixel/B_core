//! Frozen R44 held-out transfer suite.
//!
//! Do not execute this binary until the diagnostic definition-grounding suite
//! passes. Cases use fresh labels, paraphrased definitions, language switching,
//! delayed reuse, scope, and adversarial non-asserted definitions.

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
struct Binding {
    turn: usize,
    alias: &'static str,
    canonical: &'static str,
}

#[derive(Clone, Copy)]
enum FinalExpectation {
    Plan {
        intent: &'static str,
        canonical: &'static str,
        subject: Option<&'static str>,
    },
    NoPlan,
    ExplainQuoted {
        canonical: &'static str,
    },
}

struct Case {
    id: &'static str,
    category: &'static str,
    turns: Vec<(&'static str, LanguageCodeIR)>,
    bindings: Vec<Binding>,
    shared_payload: Option<(usize, usize)>,
    rejected_turn: Option<usize>,
    conflict_turn: Option<usize>,
    final_expectation: FinalExpectation,
}

fn request(id: &str, turn: u64, text: &str, language: LanguageCodeIR) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.into(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.into(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn contract(value: &Value) -> bool {
    value["schema"] == CONVERSATION_TURN_RESPONSE_SCHEMA
        && value
            .get("definition_grounding")
            .is_some_and(|item| !item.is_null())
        && value.pointer("/six_axis_integration/complete") == Some(&Value::Bool(true))
        && value.pointer("/six_axis_integration/semantic_authority") == Some(&Value::Bool(false))
        && value.pointer("/six_axis_integration/language_can_execute") == Some(&Value::Bool(false))
        && value.pointer("/grounded_realization/unsupported_claims")
            == Some(&Value::Number(0_u64.into()))
}

fn frame(value: &Value, canonical: &str) -> bool {
    value
        .pointer("/pragmatic_interpretation/compositional_analysis/frames")
        .and_then(Value::as_array)
        .is_some_and(|frames| {
            frames
                .iter()
                .any(|item| item["canonical_predicate"] == canonical)
        })
}

fn binding(values: &[Value], expected: Binding) -> bool {
    values.get(expected.turn - 1).is_some_and(|value| {
        value.pointer("/definition_grounding/disposition") == Some(&Value::String("BOUND".into()))
            && value
                .pointer("/definition_grounding/binding/alias_surface")
                .and_then(Value::as_str)
                == Some(expected.alias)
            && value
                .pointer("/definition_grounding/binding/canonical_predicate")
                .and_then(Value::as_str)
                == Some(expected.canonical)
            && value.pointer("/definition_grounding/binding/semantic_authority")
                == Some(&Value::Bool(false))
            && value.pointer("/definition_grounding/binding/external_action_execution_authorized")
                == Some(&Value::Bool(false))
            && value.get("grounded_response").is_none_or(Value::is_null)
    })
}

fn final_check(value: &Value, expected: FinalExpectation) -> bool {
    match expected {
        FinalExpectation::Plan {
            intent,
            canonical,
            subject,
        } => {
            value
                .pointer("/grounded_response/plan/intent")
                .and_then(Value::as_str)
                == Some(intent)
                && frame(value, canonical)
                && subject.is_none_or(|needle| {
                    value
                        .pointer("/grounded_response/understanding/subject")
                        .and_then(Value::as_str)
                        .is_some_and(|text| text.to_lowercase().contains(&needle.to_lowercase()))
                })
        }
        FinalExpectation::NoPlan => value.get("grounded_response").is_none_or(Value::is_null),
        FinalExpectation::ExplainQuoted { canonical } => {
            value
                .pointer("/grounded_response/plan/intent")
                .and_then(Value::as_str)
                == Some("EXPLAIN")
                && value
                    .pointer("/pragmatic_interpretation/compositional_analysis/frames")
                    .and_then(Value::as_array)
                    .is_some_and(|frames| {
                        frames.iter().any(|item| {
                            item["canonical_predicate"] == canonical
                                && item["embedded_under_quote"] == true
                                && item["external_execution_authorized"] == false
                        })
                    })
        }
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
                u64::try_from(offset + 1).expect("turn"),
                text,
                *language,
            ))
            .expect("response");
        trace.push(response.output.text.clone());
        values.push(serde_json::to_value(response).expect("json"));
    }
    let final_value = values.last().expect("final turn");
    let shared = case.shared_payload.is_none_or(|(left, right)| {
        let at = |turn: usize| {
            values
                .get(turn - 1)
                .and_then(|value| {
                    value.pointer("/definition_grounding/binding/semantic_payload_sha256")
                })
                .and_then(Value::as_str)
        };
        at(left).is_some() && at(left) == at(right)
    });
    let rejected = case.rejected_turn.is_none_or(|turn| {
        values.get(turn - 1).is_some_and(|value| {
            value
                .pointer("/definition_grounding/disposition")
                .and_then(Value::as_str)
                .is_some_and(|item| item.ends_with("_REJECTED"))
                && value
                    .pointer("/definition_grounding/binding")
                    .is_none_or(Value::is_null)
        })
    });
    let conflict = case.conflict_turn.is_none_or(|turn| {
        values.get(turn - 1).is_some_and(|value| {
            value.pointer("/definition_grounding/disposition")
                == Some(&Value::String("CONFLICT_REJECTED".into()))
        })
    });
    let pass = values.iter().all(contract)
        && case
            .bindings
            .iter()
            .copied()
            .all(|item| binding(&values, item))
        && shared
        && rejected
        && conflict
        && final_check(final_value, case.final_expectation);
    Row {
        id: case.id.into(),
        category: case.category.into(),
        pass,
        trace,
    }
}

#[allow(clippy::too_many_arguments)]
fn case(
    id: &'static str,
    category: &'static str,
    turns: Vec<(&'static str, LanguageCodeIR)>,
    bindings: Vec<Binding>,
    shared_payload: Option<(usize, usize)>,
    rejected_turn: Option<usize>,
    conflict_turn: Option<usize>,
    final_expectation: FinalExpectation,
) -> Case {
    Case {
        id,
        category,
        turns,
        bindings,
        shared_payload,
        rejected_turn,
        conflict_turn,
        final_expectation,
    }
}

fn cases() -> Vec<Case> {
    use FinalExpectation::{ExplainQuoted, NoPlan, Plan};
    use LanguageCodeIR::{English as En, Korean as Ko};
    let b = |turn, alias, canonical| Binding {
        turn,
        alias,
        canonical,
    };
    vec![
        case(
            "R44_TPARA_01",
            "definition_paraphrase",
            vec![
                ("Here, \"tavin\" means inspect.", En),
                ("Tavin the cache.", En),
            ],
            vec![b(1, "tavin", "INVESTIGATE")],
            None,
            None,
            None,
            Plan {
                intent: "INVESTIGATE",
                canonical: "INVESTIGATE",
                subject: Some("cache"),
            },
        ),
        case(
            "R44_TPARA_02",
            "definition_paraphrase",
            vec![
                ("For this chat, \"xelar\" means repair.", En),
                ("Xelar the worker.", En),
            ],
            vec![b(1, "xelar", "REPAIR")],
            None,
            None,
            None,
            Plan {
                intent: "REPAIR",
                canonical: "REPAIR",
                subject: Some("worker"),
            },
        ),
        case(
            "R44_TPARA_03",
            "definition_paraphrase",
            vec![
                ("이 대화에서는 \"해온\"을 검사하라는 뜻으로 쓸게.", Ko),
                ("로그를 해온해줘.", Ko),
            ],
            vec![b(1, "해온", "INVESTIGATE")],
            None,
            None,
            None,
            Plan {
                intent: "INVESTIGATE",
                canonical: "INVESTIGATE",
                subject: Some("로그"),
            },
        ),
        case(
            "R44_TPARA_04",
            "definition_paraphrase",
            vec![
                ("이제부터 \"다온\"은 수리하라는 뜻이야.", Ko),
                ("큐를 다온해줘.", Ko),
            ],
            vec![b(1, "다온", "REPAIR")],
            None,
            None,
            None,
            Plan {
                intent: "REPAIR",
                canonical: "REPAIR",
                subject: Some("큐"),
            },
        ),
        case(
            "R44_TCHAIN_01",
            "cross_language_alias_chain",
            vec![
                ("\"tavin\" means inspect.", En),
                ("\"소라\"는 tavin하라는 뜻이야.", Ko),
                ("캐시를 소라해줘.", Ko),
            ],
            vec![b(1, "tavin", "INVESTIGATE"), b(2, "소라", "INVESTIGATE")],
            Some((1, 2)),
            None,
            None,
            Plan {
                intent: "INVESTIGATE",
                canonical: "INVESTIGATE",
                subject: Some("캐시"),
            },
        ),
        case(
            "R44_TCHAIN_02",
            "cross_language_alias_chain",
            vec![
                ("\"다온\"은 수리하라는 뜻이야.", Ko),
                ("\"xelar\" means 다온.", En),
                ("Xelar the parser.", En),
            ],
            vec![b(1, "다온", "REPAIR"), b(2, "xelar", "REPAIR")],
            Some((1, 2)),
            None,
            None,
            Plan {
                intent: "REPAIR",
                canonical: "REPAIR",
                subject: Some("parser"),
            },
        ),
        case(
            "R44_TCHAIN_03",
            "cross_language_alias_chain",
            vec![
                ("\"pavin\" means create.", En),
                ("\"새빛\"은 pavin하라는 뜻이야.", Ko),
                ("보고서를 새빛해줘.", Ko),
            ],
            vec![b(1, "pavin", "CREATE"), b(2, "새빛", "CREATE")],
            Some((1, 2)),
            None,
            None,
            Plan {
                intent: "CREATE",
                canonical: "CREATE",
                subject: Some("보고서"),
            },
        ),
        case(
            "R44_TCHAIN_04",
            "cross_language_alias_chain",
            vec![
                ("\"지움\"은 삭제하라는 뜻이야.", Ko),
                ("\"dovel\" means 지움.", En),
                ("Dovel the cache.", En),
            ],
            vec![b(1, "지움", "DELETE"), b(2, "dovel", "DELETE")],
            Some((1, 2)),
            None,
            None,
            Plan {
                intent: "EXECUTE",
                canonical: "DELETE",
                subject: Some("cache"),
            },
        ),
        case(
            "R44_TDELAY_01",
            "delayed_cross_language_reuse",
            vec![
                ("\"tavin\" means inspect.", En),
                ("서버를 검사해.", Ko),
                ("고마워", Ko),
                ("캐시 얘기로 돌아가자.", Ko),
                ("Tavin it.", En),
            ],
            vec![b(1, "tavin", "INVESTIGATE")],
            None,
            None,
            None,
            Plan {
                intent: "INVESTIGATE",
                canonical: "INVESTIGATE",
                subject: Some("cache"),
            },
        ),
        case(
            "R44_TDELAY_02",
            "delayed_cross_language_reuse",
            vec![
                ("\"다온\"은 수리하라는 뜻이야.", Ko),
                ("Inspect the worker.", En),
                ("thanks", En),
                ("파서 얘기로 돌아가자.", Ko),
                ("그거 다온해줘.", Ko),
            ],
            vec![b(1, "다온", "REPAIR")],
            None,
            None,
            None,
            Plan {
                intent: "REPAIR",
                canonical: "REPAIR",
                subject: Some("파서"),
            },
        ),
        case(
            "R44_TDELAY_03",
            "delayed_cross_language_reuse",
            vec![
                ("\"pavin\" means create.", En),
                ("uh...", En),
                ("one moment", En),
                ("thanks", En),
                ("Pavin the summary.", En),
            ],
            vec![b(1, "pavin", "CREATE")],
            None,
            None,
            None,
            Plan {
                intent: "CREATE",
                canonical: "CREATE",
                subject: Some("summary"),
            },
        ),
        case(
            "R44_TDELAY_04",
            "delayed_cross_language_reuse",
            vec![
                ("\"지움\"은 삭제하라는 뜻이야.", Ko),
                ("음...", Ko),
                ("잠깐", Ko),
                ("고마워", Ko),
                ("임시 파일을 지움해줘.", Ko),
            ],
            vec![b(1, "지움", "DELETE")],
            None,
            None,
            None,
            Plan {
                intent: "EXECUTE",
                canonical: "DELETE",
                subject: Some("임시 파일"),
            },
        ),
        case(
            "R44_TSCOPE_01",
            "learned_scope_transfer",
            vec![
                ("\"xelar\" means repair.", En),
                ("Do not xelar the queue; explain why it failed.", En),
            ],
            vec![b(1, "xelar", "REPAIR")],
            None,
            None,
            None,
            NoPlan,
        ),
        case(
            "R44_TSCOPE_02",
            "learned_scope_transfer",
            vec![
                ("\"다온\"은 수리하라는 뜻이야.", Ko),
                ("캐시를 다온하지 말고 왜 느린지 설명해줘.", Ko),
            ],
            vec![b(1, "다온", "REPAIR")],
            None,
            None,
            None,
            NoPlan,
        ),
        case(
            "R44_TSCOPE_03",
            "learned_scope_transfer",
            vec![
                ("\"dovel\" means delete.", En),
                ("Explain 'dovel the log'.", En),
            ],
            vec![b(1, "dovel", "DELETE")],
            None,
            None,
            None,
            ExplainQuoted {
                canonical: "DELETE",
            },
        ),
        case(
            "R44_TSCOPE_04",
            "learned_scope_transfer",
            vec![
                ("\"지움\"은 삭제하라는 뜻이야.", Ko),
                ("'백업을 지움해'라는 표현을 설명해줘.", Ko),
            ],
            vec![b(1, "지움", "DELETE")],
            None,
            None,
            None,
            ExplainQuoted {
                canonical: "DELETE",
            },
        ),
        case(
            "R44_TREJECT_01",
            "adversarial_definition",
            vec![("\"delete\" means inspect.", En), ("Delete the cache.", En)],
            vec![],
            None,
            None,
            Some(1),
            Plan {
                intent: "EXECUTE",
                canonical: "DELETE",
                subject: Some("cache"),
            },
        ),
        case(
            "R44_TREJECT_02",
            "adversarial_definition",
            vec![
                ("\"mepra\" means frobnicate.", En),
                ("Mepra the cache.", En),
            ],
            vec![],
            None,
            Some(1),
            None,
            NoPlan,
        ),
        case(
            "R44_TREJECT_03",
            "adversarial_definition",
            vec![
                ("민수가 '소라라는 말은 삭제하라는 뜻이야'라고 말했어.", Ko),
                ("캐시를 소라해줘.", Ko),
            ],
            vec![],
            None,
            Some(1),
            None,
            NoPlan,
        ),
        case(
            "R44_TREJECT_04",
            "adversarial_definition",
            vec![
                ("\"소라\"는 삭제하라는 뜻이야?", Ko),
                ("캐시를 소라해줘.", Ko),
            ],
            vec![],
            None,
            Some(1),
            None,
            NoPlan,
        ),
    ]
}

fn main() {
    let rows = cases().into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    println!("{}", serde_json::to_string_pretty(&rows).expect("rows"));
    println!("R44_TRANSFER_PASSED={passed}/{}", rows.len());
    if passed != rows.len() {
        std::process::exit(1);
    }
}
