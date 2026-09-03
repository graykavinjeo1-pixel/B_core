//! Frozen R54 held-out transfer suite. These surfaces are disjoint from the
//! diagnostic and are not executed until the diagnostic product repair passes.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DecisionBranchActionIR, LanguageCodeIR, SpeechActIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
struct Turn {
    text: &'static str,
    language: LanguageCodeIR,
}

enum Expectation {
    Continuation {
        topic: &'static str,
        task: &'static str,
        forbidden: &'static [&'static str],
        benefit: &'static str,
    },
    RestoredGateSet {
        topic: &'static str,
        task: &'static str,
        benefit: &'static str,
        gate_count: usize,
    },
    MissingTask {
        topic: &'static str,
        forbidden: &'static [&'static str],
    },
}

struct Case {
    id: &'static str,
    category: &'static str,
    turns: &'static [Turn],
    restoration_turn: usize,
    expectation: Expectation,
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    restored_topic: Option<String>,
    task: Option<String>,
    benefit: Option<String>,
    topic_gate_count: usize,
    unresolved_bindings: Vec<String>,
    disposition: String,
    contract_valid: bool,
    authority_violation: bool,
    unsupported_facts: usize,
    pass: bool,
}

#[derive(Serialize)]
struct Summary {
    schema: &'static str,
    suite: &'static str,
    total: usize,
    passed: usize,
    failed: usize,
    response_contracts_valid: usize,
    authority_violations: usize,
    unsupported_explanation_facts: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn request(id: &str, index: u64, turn: Turn) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: index,
        request_id: format!("{id}-{index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: turn.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(turn.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn has(value: &str, term: &str) -> bool {
    value.to_lowercase().contains(&term.to_lowercase())
}

fn main() {
    use LanguageCodeIR::{English, Korean};
    const CASES: &[Case] = &[
        Case {
            id: "R54_H01",
            category: "novel_english_surface",
            turns: &[
                Turn { text: "Switch to parser.", language: English },
                Turn { text: "We are repairing the parser.", language: English },
                Turn { text: "Switch to scheduler.", language: English },
                Turn { text: "We are testing the scheduler.", language: English },
                Turn { text: "Resume parser.", language: English },
                Turn { text: "Keep at it only when fresh trials reduce production failures; otherwise ask whether to stop.", language: English },
            ],
            restoration_turn: 5,
            expectation: Expectation::Continuation { topic: "parser", task: "repairing", forbidden: &["scheduler", "testing"], benefit: "failure" },
        },
        Case {
            id: "R54_H02",
            category: "novel_korean_surface",
            turns: &[
                Turn { text: "인덱스 주제로 전환해.", language: Korean },
                Turn { text: "현재 인덱스를 재설계하는 중이야.", language: Korean },
                Turn { text: "라우터 주제로 전환해.", language: Korean },
                Turn { text: "현재 라우터를 조사하는 중이야.", language: Korean },
                Turn { text: "인덱스 주제로 복귀해.", language: Korean },
                Turn { text: "새 검증에서 실제 장애가 줄 때만 그 일을 이어가. 아니면 중단할지 물어.", language: Korean },
            ],
            restoration_turn: 5,
            expectation: Expectation::Continuation { topic: "인덱스", task: "재설계", forbidden: &["라우터", "조사"], benefit: "장애" },
        },
        Case {
            id: "R54_H03",
            category: "indexed_two_topic_return",
            turns: &[
                Turn { text: "Switch to parser.", language: English },
                Turn { text: "We are refactoring the parser.", language: English },
                Turn { text: "Switch to scheduler.", language: English },
                Turn { text: "We are repairing the scheduler.", language: English },
                Turn { text: "Switch to journal.", language: English },
                Turn { text: "We are testing the journal.", language: English },
                Turn { text: "Return to the topic from two topics ago.", language: English },
                Turn { text: "Continue it only if cold runs expand real coverage; otherwise report and ask before stopping.", language: English },
            ],
            restoration_turn: 7,
            expectation: Expectation::Continuation { topic: "parser", task: "refactoring", forbidden: &["scheduler", "journal", "repairing", "testing"], benefit: "coverage" },
        },
        Case {
            id: "R54_H04",
            category: "indexed_korean_return",
            turns: &[
                Turn { text: "인덱스 주제로 전환해.", language: Korean },
                Turn { text: "현재 인덱스를 테스트하는 중이야.", language: Korean },
                Turn { text: "라우터 주제로 전환해.", language: Korean },
                Turn { text: "현재 라우터를 수리하는 중이야.", language: Korean },
                Turn { text: "스케줄러 주제로 전환해.", language: Korean },
                Turn { text: "현재 스케줄러를 조사하는 중이야.", language: Korean },
                Turn { text: "두 주제 전으로 돌아가자.", language: Korean },
                Turn { text: "실제 장애가 줄 때만 그 일을 계속해. 아니면 중단할지 물어.", language: Korean },
            ],
            restoration_turn: 7,
            expectation: Expectation::Continuation { topic: "인덱스", task: "테스트", forbidden: &["라우터", "스케줄러", "수리", "조사"], benefit: "장애" },
        },
        Case {
            id: "R54_H05",
            category: "two_pending_gates_restore_first",
            turns: &[
                Turn { text: "Switch to cache.", language: English },
                Turn { text: "We are refactoring the cache.", language: English },
                Turn { text: "Continue it only if fresh trials expand production coverage; otherwise ask whether to stop.", language: English },
                Turn { text: "Switch to queue.", language: English },
                Turn { text: "We are repairing the queue.", language: English },
                Turn { text: "Continue it only if cold runs reduce production failures; otherwise ask whether to stop.", language: English },
                Turn { text: "Return to cache.", language: English },
            ],
            restoration_turn: 7,
            expectation: Expectation::RestoredGateSet { topic: "cache", task: "refactoring", benefit: "coverage", gate_count: 2 },
        },
        Case {
            id: "R54_H06",
            category: "cross_language_pending_gate_restore",
            turns: &[
                Turn { text: "백업 주제로 전환해.", language: Korean },
                Turn { text: "현재 백업을 재설계하는 중이야.", language: Korean },
                Turn { text: "실제 장애가 줄 때만 그 일을 계속해. 아니면 중단할지 물어.", language: Korean },
                Turn { text: "Switch to worker.", language: English },
                Turn { text: "We are testing the worker.", language: English },
                Turn { text: "Keep at it only when fresh runs expand production coverage; otherwise ask whether to stop.", language: English },
                Turn { text: "Return to the backup.", language: English },
            ],
            restoration_turn: 7,
            expectation: Expectation::RestoredGateSet { topic: "backup", task: "재설계", benefit: "장애", gate_count: 2 },
        },
        Case {
            id: "R54_H07",
            category: "similar_surface_identity_is_exact",
            turns: &[
                Turn { text: "Switch to cache.", language: English },
                Turn { text: "We are repairing the cache.", language: English },
                Turn { text: "Switch to cache policy.", language: English },
                Turn { text: "We are testing the cache policy.", language: English },
                Turn { text: "Return to cache.", language: English },
                Turn { text: "Keep at it only if fresh trials reduce real failures; otherwise ask whether to stop.", language: English },
            ],
            restoration_turn: 5,
            expectation: Expectation::Continuation { topic: "cache", task: "repairing", forbidden: &["policy", "testing"], benefit: "failure" },
        },
        Case {
            id: "R54_H08",
            category: "novel_unseen_topic_fails_closed",
            turns: &[
                Turn { text: "Switch to parser.", language: English },
                Turn { text: "We are repairing the parser.", language: English },
                Turn { text: "Switch to telemetry.", language: English },
                Turn { text: "Continue it only if fresh checks expand real coverage; otherwise ask whether to stop.", language: English },
            ],
            restoration_turn: 3,
            expectation: Expectation::MissingTask { topic: "telemetry", forbidden: &["parser", "repairing"] },
        },
    ];

    let mut rows = Vec::new();
    for case in CASES {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut responses = Vec::new();
        let mut contract_valid = true;
        for (offset, turn) in case.turns.iter().copied().enumerate() {
            let request = request(case.id, u64::try_from(offset + 1).expect("turn"), turn);
            match api.process_conversation_turn(&request) {
                Ok(response) => {
                    contract_valid &= response.validate_against(&request);
                    responses.push(response);
                }
                Err(_) => break,
            }
        }
        let restored = case
            .restoration_turn
            .checked_sub(1)
            .and_then(|index| responses.get(index));
        let final_response = responses.last();
        let restored_topic = restored
            .and_then(|response| response.conversation_state.active_topics.first())
            .filter(|topic| topic.explicitly_activated)
            .map(|topic| topic.surface.clone());
        let continuation = final_response
            .and_then(|response| response.pragmatic_interpretation.continuation_gate.as_ref());
        let pending = restored
            .and_then(|response| response.pragmatic_state.pending_continuation_gate.as_ref());
        let topic_gate_count = restored.map_or(0, |response| {
            response
                .pragmatic_state
                .topic_pending_continuation_gates
                .len()
        });
        let (task, benefit) = continuation
            .map(|gate| {
                (
                    Some(gate.current_task.clone()),
                    Some(gate.required_benefit.clone()),
                )
            })
            .unwrap_or_else(|| {
                pending
                    .map(|gate| (Some(gate.task.clone()), Some(gate.required_benefit.clone())))
                    .unwrap_or((None, None))
            });
        let topic_matches = |term: &str| {
            restored_topic
                .as_deref()
                .is_some_and(|surface| has(surface, term))
        };
        let expectation_pass = match &case.expectation {
            Expectation::Continuation {
                topic,
                task: expected_task,
                forbidden,
                benefit: expected_benefit,
            } => {
                topic_matches(topic)
                    && continuation.is_some_and(|gate| {
                        has(&gate.current_task, expected_task)
                            && forbidden.iter().all(|term| !has(&gate.current_task, term))
                            && has(&gate.required_benefit, expected_benefit)
                            && gate.verification_required
                            && gate.positive_action == DecisionBranchActionIR::ContinueCurrentWork
                    })
                    && final_response.is_some_and(|response| {
                        response.pragmatic_interpretation.speech_act
                            == SpeechActIR::ConditionalContinuation
                    })
            }
            Expectation::RestoredGateSet {
                topic,
                task: expected_task,
                benefit: expected_benefit,
                gate_count,
            } => {
                topic_matches(topic)
                    && pending.is_some_and(|gate| {
                        has(&gate.task, expected_task)
                            && has(&gate.required_benefit, expected_benefit)
                    })
                    && topic_gate_count == *gate_count
            }
            Expectation::MissingTask { topic, forbidden } => {
                topic_matches(topic)
                    && continuation.is_none()
                    && forbidden
                        .iter()
                        .all(|term| task.as_deref().is_none_or(|value| !has(value, term)))
                    && final_response.is_some_and(|response| {
                        response.disposition == ConversationTurnDispositionIR::ClarificationRequired
                            && response
                                .pragmatic_interpretation
                                .unresolved_bindings
                                .iter()
                                .any(|binding| binding == "CURRENT_TASK")
                    })
            }
        };
        let authority_violation = responses.iter().any(|response| {
            response.language_cortex_integration.semantic_authority
                || response.language_cortex_integration.language_can_execute
                || response
                    .language_cortex_integration
                    .external_action_executed
                || response.action_state_analysis.external_action_executed
        });
        let unsupported_facts = responses
            .iter()
            .map(|response| {
                response
                    .language_cortex_integration
                    .unsupported_explanation_facts
            })
            .sum();
        let safe = responses.iter().all(|response| {
            response.grounded_realization.validate()
                && response.grounded_realization.realized_text == response.output.text
        });
        let pass = responses.len() == case.turns.len()
            && contract_valid
            && safe
            && expectation_pass
            && !authority_violation
            && unsupported_facts == 0;
        rows.push(Row {
            id: case.id.to_string(),
            category: case.category.to_string(),
            restored_topic,
            task,
            benefit,
            topic_gate_count,
            unresolved_bindings: final_response.map_or_else(Vec::new, |response| {
                response
                    .pragmatic_interpretation
                    .unresolved_bindings
                    .clone()
            }),
            disposition: final_response
                .map(|response| format!("{:?}", response.disposition))
                .unwrap_or_else(|| "MISSING".to_string()),
            contract_valid,
            authority_violation,
            unsupported_facts,
            pass,
        });
    }
    let total = rows.len();
    let passed = rows.iter().filter(|row| row.pass).count();
    let response_contracts_valid = rows.iter().filter(|row| row.contract_valid).count();
    let authority_violations = rows.iter().filter(|row| row.authority_violation).count();
    let unsupported_explanation_facts = rows.iter().map(|row| row.unsupported_facts).sum();
    println!(
        "{}",
        serde_json::to_string(&Summary {
            schema: "B_CORE_R54_TOPIC_SCOPED_PRAGMATIC_STATE_TRANSFER_CANARY_1",
            suite: "R54_TOPIC_SCOPED_PRAGMATIC_STATE_HELDOUT",
            total,
            passed,
            failed: total - passed,
            response_contracts_valid,
            authority_violations,
            unsupported_explanation_facts,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            network_calls: 0,
            recursive_source_mutations: 0,
            rows,
        })
        .expect("summary")
    );
    if passed != total {
        std::process::exit(1);
    }
}
