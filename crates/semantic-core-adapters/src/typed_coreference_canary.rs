//! Frozen R13-RUN-0001 diagnostic suite for typed, long-distance coreference.
//!
//! The fixtures intentionally use names and surfaces that are absent from the
//! implementation.  They test structural binding, not sentence dispatch.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
enum Expectation {
    Resolve {
        target: &'static str,
        binding: &'static str,
    },
    Clarify,
    Unchanged {
        text: &'static str,
    },
}

struct Case {
    case_id: String,
    turns: Vec<String>,
    expectation: Expectation,
}

#[derive(Debug, Serialize)]
struct Row {
    case_id: String,
    turn_count: usize,
    disposition: ConversationTurnDispositionIR,
    resolved_semantic_text: String,
    binding_kinds: Vec<String>,
    target_present: bool,
    ambiguity_preserved: bool,
    typed_entity_memory: Vec<String>,
    event_memory: Vec<String>,
    pass: bool,
}

fn request(conversation_id: &str, turn_index: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        turn_index,
        request_id: format!("{conversation_id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(if text.is_ascii() {
            LanguageCodeIR::English
        } else {
            LanguageCodeIR::Korean
        }),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn fillers(count: usize) -> Vec<String> {
    [
        "inspect the file",
        "check the folder",
        "review the report",
        "analyze the code",
        "inspect the repository",
        "review the plan",
        "check the document",
        "analyze the project",
        "inspect the error",
    ]
    .into_iter()
    .cycle()
    .take(count)
    .map(str::to_string)
    .collect()
}

fn actor_case(
    case_id: &str,
    introduction: &str,
    distance: usize,
    reference: &str,
    target: &'static str,
    binding: &'static str,
) -> Case {
    let mut turns = vec![introduction.to_string()];
    turns.extend(fillers(distance));
    turns.push(reference.to_string());
    Case {
        case_id: case_id.to_string(),
        turns,
        expectation: Expectation::Resolve { target, binding },
    }
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (index, (name, pronoun)) in [
        ("Avery", "She"),
        ("Bennett", "He"),
        ("Celeste", "she"),
        ("Dorian", "he"),
    ]
    .into_iter()
    .enumerate()
    {
        cases.push(actor_case(
            &format!("EN_PERSON_LONG_{index:02}"),
            &format!("{name} says that the cache is stale"),
            6 + index,
            &format!("{pronoun} later corrected the report"),
            name,
            "TypedEntityReference",
        ));
        cases.push(actor_case(
            &format!("EN_BELIEF_HOLDER_{index:02}"),
            &format!("{name} claims that the worker is blocked"),
            5 + index,
            &format!(
                "explain {} claim",
                if pronoun == "He" || pronoun == "he" {
                    "his"
                } else {
                    "her"
                }
            ),
            name,
            "BeliefHolderReference",
        ));
    }
    for (index, (name, pronoun, possessive)) in [
        ("가람", "그녀가", "그녀의"),
        ("나루", "그가", "그의"),
        ("다온", "그녀가", "그녀의"),
        ("라온", "그가", "그의"),
    ]
    .into_iter()
    .enumerate()
    {
        cases.push(actor_case(
            &format!("KO_PERSON_LONG_{index:02}"),
            &format!("{name}은 캐시가 오래됐다고 말했다"),
            6 + index,
            &format!("{pronoun} 나중에 보고서를 수정했다"),
            name,
            "TypedEntityReference",
        ));
        cases.push(actor_case(
            &format!("KO_BELIEF_HOLDER_{index:02}"),
            &format!("{name}은 워커가 멈췄다고 주장했다"),
            5 + index,
            &format!("{possessive} 주장을 설명해"),
            name,
            "BeliefHolderReference",
        ));
    }

    cases.extend([
        actor_case(
            "CROSS_EN_TO_KO_PERSON",
            "Mirelle says that the deployment is risky",
            7,
            "그녀가 나중에 계획을 수정했다",
            "Mirelle",
            "TypedEntityReference",
        ),
        actor_case(
            "CROSS_KO_TO_EN_PERSON",
            "보람은 배포가 위험하다고 말했다",
            7,
            "She later corrected the plan",
            "보람",
            "TypedEntityReference",
        ),
        actor_case(
            "CROSS_EN_TO_KO_BELIEF",
            "Nolan believes that the parser might fail",
            6,
            "그의 믿음을 설명해",
            "Nolan",
            "BeliefHolderReference",
        ),
        actor_case(
            "CROSS_KO_TO_EN_BELIEF",
            "서린은 파서가 실패할 수 있다고 믿는다",
            6,
            "explain her belief",
            "서린",
            "BeliefHolderReference",
        ),
        actor_case(
            "EN_TYPED_NONPERSON",
            "the Oriole service was reviewed by Petra",
            5,
            "inspect that service again",
            "Oriole service",
            "TypedEntityReference",
        ),
        actor_case(
            "KO_TYPED_NONPERSON",
            "누리 서비스를 확인해",
            5,
            "그 서비스를 다시 확인해",
            "누리 서비스",
            "TypedEntityReference",
        ),
        actor_case(
            "EN_EVENT_LONG_DISTANCE",
            "inspect the migration operation",
            8,
            "explain that migration operation",
            "migration",
            "EventReference",
        ),
        actor_case(
            "KO_EVENT_LONG_DISTANCE",
            "마이그레이션 작업을 확인해",
            8,
            "그 마이그레이션 작업을 설명해",
            "마이그레이션",
            "EventReference",
        ),
    ]);

    cases.extend([
        Case {
            case_id: "EN_AMBIGUOUS_TWO_PEOPLE".to_string(),
            turns: vec![
                "Quinn says that the build failed".to_string(),
                "Rowan says that the cache failed".to_string(),
                "She corrected the report".to_string(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "KO_AMBIGUOUS_TWO_PEOPLE".to_string(),
            turns: vec![
                "마루는 빌드가 실패했다고 말했다".to_string(),
                "아라는 캐시가 실패했다고 말했다".to_string(),
                "그녀가 보고서를 수정했다".to_string(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "EN_UNBOUND_PERSON".to_string(),
            turns: vec!["She corrected the report".to_string()],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "KO_UNBOUND_PERSON".to_string(),
            turns: vec!["그가 보고서를 수정했다".to_string()],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "EN_AMBIGUOUS_BELIEF_HOLDER".to_string(),
            turns: vec![
                "Sable claims that the server is slow".to_string(),
                "Tobin claims that the cache is stale".to_string(),
                "explain her claim".to_string(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "KO_AMBIGUOUS_BELIEF_HOLDER".to_string(),
            turns: vec![
                "유라는 서버가 느리다고 주장했다".to_string(),
                "해온은 캐시가 오래됐다고 주장했다".to_string(),
                "그녀의 주장을 설명해".to_string(),
            ],
            expectation: Expectation::Clarify,
        },
    ]);
    cases.extend(run2_blind_cases());
    cases.extend(run3_blind_cases());
    cases
}

fn run3_blind_cases() -> Vec<Case> {
    vec![
        actor_case(
            "R13_RUN3_EN_PERSON_SYSTEM_DISTRACTOR",
            "Rhea says that the compiler is overloaded",
            11,
            "She corrected the document afterward",
            "Rhea",
            "TypedEntityReference",
        ),
        actor_case(
            "R13_RUN3_KO_PERSON_SCHEDULER_DISTRACTOR",
            "보미는 스케줄러가 과부하라고 말했다",
            11,
            "그녀가 나중에 문서를 수정했다",
            "보미",
            "TypedEntityReference",
        ),
        actor_case(
            "R13_RUN3_EN_PERSON_WORKER_DISTRACTOR",
            "Silas reports that the worker is unavailable",
            12,
            "He corrected the report afterward",
            "Silas",
            "TypedEntityReference",
        ),
        actor_case(
            "R13_RUN3_KO_PERSON_SERVER_DISTRACTOR",
            "태윤은 서버가 느리다고 보고했다",
            12,
            "그가 나중에 보고서를 수정했다",
            "태윤",
            "TypedEntityReference",
        ),
        actor_case(
            "R13_RUN3_EN_BELIEF_HOLDER",
            "Vesper believes that the compiler might stall",
            10,
            "explain her belief",
            "Vesper",
            "BeliefHolderReference",
        ),
        actor_case(
            "R13_RUN3_KO_BELIEF_HOLDER",
            "윤슬은 스케줄러가 멈출 수 있다고 믿는다",
            10,
            "그녀의 믿음을 설명해",
            "윤슬",
            "BeliefHolderReference",
        ),
        actor_case(
            "R13_RUN3_CROSS_EN_KO",
            "Wren claims that the parser is unstable",
            10,
            "그의 주장을 설명해",
            "Wren",
            "BeliefHolderReference",
        ),
        actor_case(
            "R13_RUN3_CROSS_KO_EN",
            "해솔은 워커가 불안정하다고 주장했다",
            10,
            "explain his claim",
            "해솔",
            "BeliefHolderReference",
        ),
        actor_case(
            "R13_RUN3_EN_SYSTEM_DESCRIPTOR",
            "inspect the Meridian service",
            11,
            "inspect that service again",
            "Meridian service",
            "TypedEntityReference",
        ),
        actor_case(
            "R13_RUN3_KO_SYSTEM_DESCRIPTOR",
            "온새미 서비스를 확인해",
            11,
            "그 서비스를 다시 확인해",
            "온새미 서비스",
            "TypedEntityReference",
        ),
        actor_case(
            "R13_RUN3_EN_EVENT_DESCRIPTOR",
            "inspect the prism migration operation",
            12,
            "explain that prism migration operation",
            "prism migration",
            "EventReference",
        ),
        actor_case(
            "R13_RUN3_KO_EVENT_DESCRIPTOR",
            "새결 마이그레이션 작업을 확인해",
            12,
            "그 새결 마이그레이션 작업을 설명해",
            "새결 마이그레이션",
            "EventReference",
        ),
        Case {
            case_id: "R13_RUN3_PERSON_TIE_EN".to_string(),
            turns: vec![
                "Xanthe says that the server is ready".to_string(),
                "Yarrow says that the worker is ready".to_string(),
                "She corrected the document".to_string(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "R13_RUN3_PERSON_TIE_KO".to_string(),
            turns: vec![
                "다별은 서버가 준비됐다고 말했다".to_string(),
                "로한은 워커가 준비됐다고 말했다".to_string(),
                "그가 문서를 수정했다".to_string(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "R13_RUN3_UNBOUND_PERSON".to_string(),
            turns: vec!["He corrected the document".to_string()],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "R13_RUN3_QUOTED_PERSON".to_string(),
            turns: vec![
                "Zephyr says that the server is ready".to_string(),
                "quote ‘he corrected the document’".to_string(),
            ],
            expectation: Expectation::Unchanged {
                text: "quote ‘he corrected the document’",
            },
        },
    ]
}

fn run2_blind_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (index, (name, pronoun, possessive)) in [
        ("Elara", "She", "her"),
        ("Hadrian", "He", "his"),
        ("Isolde", "She", "her"),
    ]
    .into_iter()
    .enumerate()
    {
        cases.push(actor_case(
            &format!("R13_RUN2_EN_PERSON_{index:02}"),
            &format!("{name} reports that the scheduler is delayed"),
            8 + index,
            &format!("{pronoun} corrected the document afterward"),
            name,
            "TypedEntityReference",
        ));
        cases.push(actor_case(
            &format!("R13_RUN2_EN_BELIEF_{index:02}"),
            &format!("{name} believes that the index might be stale"),
            7 + index,
            &format!("explain {possessive} belief"),
            name,
            "BeliefHolderReference",
        ));
    }
    for (index, (name, pronoun, possessive)) in [
        ("여울", "그녀가", "그녀의"),
        ("시온", "그가", "그의"),
        ("하람", "그녀가", "그녀의"),
    ]
    .into_iter()
    .enumerate()
    {
        cases.push(actor_case(
            &format!("R13_RUN2_KO_PERSON_{index:02}"),
            &format!("{name}은 스케줄러가 늦는다고 보고했다"),
            8 + index,
            &format!("{pronoun} 나중에 문서를 수정했다"),
            name,
            "TypedEntityReference",
        ));
        cases.push(actor_case(
            &format!("R13_RUN2_KO_BELIEF_{index:02}"),
            &format!("{name}은 인덱스가 오래됐을 수 있다고 믿는다"),
            7 + index,
            &format!("{possessive} 믿음을 설명해"),
            name,
            "BeliefHolderReference",
        ));
    }
    cases.extend([
        actor_case(
            "R13_RUN2_CROSS_EN_KO_PERSON",
            "Juniper claims that the queue is saturated",
            9,
            "그녀가 나중에 계획을 수정했다",
            "Juniper",
            "TypedEntityReference",
        ),
        actor_case(
            "R13_RUN2_CROSS_KO_EN_PERSON",
            "초롱은 큐가 가득 찼다고 주장했다",
            9,
            "She corrected the plan afterward",
            "초롱",
            "TypedEntityReference",
        ),
        actor_case(
            "R13_RUN2_CROSS_EN_KO_BELIEF",
            "Leander believes that the mirror might lag",
            8,
            "그의 믿음을 설명해",
            "Leander",
            "BeliefHolderReference",
        ),
        actor_case(
            "R13_RUN2_CROSS_KO_EN_BELIEF",
            "다솜은 미러가 느릴 수 있다고 믿는다",
            8,
            "explain her belief",
            "다솜",
            "BeliefHolderReference",
        ),
        actor_case(
            "R13_RUN2_EN_SYSTEM_DESCRIPTOR",
            "the Kestrel service was reviewed by Uma",
            9,
            "inspect that service again",
            "Kestrel service",
            "TypedEntityReference",
        ),
        actor_case(
            "R13_RUN2_KO_SYSTEM_DESCRIPTOR",
            "하늘 서비스를 확인해",
            9,
            "그 서비스를 다시 확인해",
            "하늘 서비스",
            "TypedEntityReference",
        ),
        actor_case(
            "R13_RUN2_EN_EVENT_DESCRIPTOR",
            "inspect the lattice migration operation",
            10,
            "explain that lattice migration operation",
            "lattice migration",
            "EventReference",
        ),
        actor_case(
            "R13_RUN2_KO_EVENT_DESCRIPTOR",
            "청명 마이그레이션 작업을 확인해",
            10,
            "그 청명 마이그레이션 작업을 설명해",
            "청명 마이그레이션",
            "EventReference",
        ),
        actor_case(
            "R13_RUN2_OUT_OF_WINDOW_EN",
            "Maribel says that the shard is unavailable",
            17,
            "She corrected the report",
            "__MUST_NOT_RESOLVE__",
            "__NO_BINDING__",
        ),
        Case {
            case_id: "R13_RUN2_AMBIGUOUS_PEOPLE_EN".to_string(),
            turns: vec![
                "Noemi says that the build is blocked".to_string(),
                "Ophelia says that the test is blocked".to_string(),
                "She corrected the report".to_string(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "R13_RUN2_AMBIGUOUS_PEOPLE_KO".to_string(),
            turns: vec![
                "이든은 빌드가 멈췄다고 말했다".to_string(),
                "지안은 테스트가 멈췄다고 말했다".to_string(),
                "그가 보고서를 수정했다".to_string(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "R13_RUN2_AMBIGUOUS_SYSTEM_EN".to_string(),
            turns: vec![
                "inspect the Falcon service".to_string(),
                "inspect the Heron service".to_string(),
                "inspect that service again".to_string(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "R13_RUN2_AMBIGUOUS_SYSTEM_KO".to_string(),
            turns: vec![
                "한빛 서비스를 확인해".to_string(),
                "새롬 서비스를 확인해".to_string(),
                "그 서비스를 다시 확인해".to_string(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "R13_RUN2_UNBOUND_BELIEF_EN".to_string(),
            turns: vec!["explain her belief".to_string()],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "R13_RUN2_UNBOUND_BELIEF_KO".to_string(),
            turns: vec!["그의 믿음을 설명해".to_string()],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "R13_RUN2_QUOTED_PERSON_EN".to_string(),
            turns: vec![
                "Phaedra says that the build is blocked".to_string(),
                "quote ‘she corrected the report’".to_string(),
            ],
            expectation: Expectation::Unchanged {
                text: "quote ‘she corrected the report’",
            },
        },
        Case {
            case_id: "R13_RUN2_QUOTED_PERSON_KO".to_string(),
            turns: vec![
                "은별은 빌드가 멈췄다고 말했다".to_string(),
                "‘그녀가 보고서를 수정했다’라고 인용해".to_string(),
            ],
            expectation: Expectation::Unchanged {
                text: "‘그녀가 보고서를 수정했다’라고 인용해",
            },
        },
    ]);
    if let Some(case) = cases
        .iter_mut()
        .find(|case| case.case_id == "R13_RUN2_OUT_OF_WINDOW_EN")
    {
        case.expectation = Expectation::Clarify;
    }
    cases
}

fn main() {
    let mut rows = Vec::new();
    for case in cases() {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut final_response = None;
        for (index, turn) in case.turns.iter().enumerate() {
            final_response = Some(
                api.process_conversation_turn(&request(
                    &case.case_id,
                    u64::try_from(index + 1).expect("turn index"),
                    turn,
                ))
                .expect("conversation turn"),
            );
        }
        let response = final_response.expect("non-empty fixture");
        let binding_kinds = response
            .reference_resolution
            .discourse_bindings
            .iter()
            .map(|binding| format!("{:?}", binding.kind))
            .collect::<Vec<_>>();
        let (target_present, ambiguity_preserved, pass) = match case.expectation {
            Expectation::Resolve { target, binding } => {
                let target_present = response
                    .reference_resolution
                    .resolved_semantic_text
                    .to_lowercase()
                    .contains(&target.to_lowercase());
                let binding_present = binding_kinds.iter().any(|kind| kind == binding);
                (
                    target_present,
                    response
                        .reference_resolution
                        .ambiguous_reference_surfaces
                        .is_empty(),
                    target_present
                        && binding_present
                        && response.disposition == ConversationTurnDispositionIR::Grounded,
                )
            }
            Expectation::Clarify => {
                let ambiguity_preserved = !response
                    .reference_resolution
                    .ambiguous_reference_surfaces
                    .is_empty();
                (
                    false,
                    ambiguity_preserved,
                    ambiguity_preserved
                        && response.disposition
                            == ConversationTurnDispositionIR::ClarificationRequired,
                )
            }
            Expectation::Unchanged { text } => {
                let no_typed_binding = binding_kinds
                    .iter()
                    .all(|kind| kind != "TypedEntityReference" && kind != "BeliefHolderReference");
                let unchanged = response.reference_resolution.resolved_semantic_text == text;
                (
                    false,
                    response
                        .reference_resolution
                        .ambiguous_reference_surfaces
                        .is_empty(),
                    unchanged && no_typed_binding,
                )
            }
        };
        rows.push(Row {
            case_id: case.case_id,
            turn_count: case.turns.len(),
            disposition: response.disposition,
            resolved_semantic_text: response.reference_resolution.resolved_semantic_text,
            binding_kinds,
            target_present,
            ambiguity_preserved,
            typed_entity_memory: response
                .conversation_state
                .active_typed_entities
                .iter()
                .map(|entity| format!("{:?}:{}", entity.kind, entity.canonical_surface))
                .collect(),
            event_memory: response
                .conversation_state
                .active_discourse_referents
                .iter()
                .filter(|referent| format!("{:?}", referent.kind) == "Event")
                .map(|referent| referent.semantic_summary.clone())
                .collect(),
            pass,
        });
    }
    println!("{}", serde_json::to_string(&rows).expect("serialize rows"));
    if rows.iter().any(|row| !row.pass) {
        std::process::exit(1);
    }
}
