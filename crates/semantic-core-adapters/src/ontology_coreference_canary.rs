//! Frozen R14-RUN-0001 ontology-mediated coreference suite.
//!
//! Surface forms in the reference turn intentionally differ from those in
//! the antecedent. Passing requires a typed ontology path, not substring
//! identity or latest-turn dispatch.

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
    case_id: &'static str,
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
    ontology_evidence: Vec<String>,
    ambiguous_surfaces: Vec<String>,
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
        "review the plan",
        "check the folder",
        "analyze the report",
        "inspect the repository",
        "review the document",
    ]
    .into_iter()
    .cycle()
    .take(count)
    .map(str::to_string)
    .collect()
}

fn long_case(
    case_id: &'static str,
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
        case_id,
        turns,
        expectation: Expectation::Resolve { target, binding },
    }
}

fn cases() -> Vec<Case> {
    vec![
        long_case(
            "EN_ENTITY_SERVICE_APPLICATION",
            "inspect the Meridian service",
            7,
            "review that application",
            "Meridian service",
            "OntologyEntityReference",
        ),
        long_case(
            "EN_ENTITY_REPORT_DOCUMENT",
            "inspect the Atlas report",
            8,
            "review that document",
            "Atlas report",
            "OntologyEntityReference",
        ),
        long_case(
            "EN_ENTITY_REPOSITORY_CODEBASE",
            "inspect the Orchid repository",
            9,
            "review that codebase",
            "Orchid repository",
            "OntologyEntityReference",
        ),
        long_case(
            "EN_ENTITY_CACHE_STORAGE_LAYER",
            "inspect the Nimbus cache",
            7,
            "review that storage layer",
            "Nimbus cache",
            "OntologyEntityReference",
        ),
        long_case(
            "EN_ENTITY_FOLDER_DIRECTORY",
            "inspect the Saffron folder",
            8,
            "review that directory",
            "Saffron folder",
            "OntologyEntityReference",
        ),
        long_case(
            "KO_ENTITY_SERVICE_APPLICATION",
            "누리 서비스를 확인해",
            7,
            "그 애플리케이션을 검토해",
            "누리 서비스",
            "OntologyEntityReference",
        ),
        long_case(
            "KO_ENTITY_REPORT_DOCUMENT",
            "다온 보고서를 확인해",
            8,
            "그 문서를 검토해",
            "다온 보고서",
            "OntologyEntityReference",
        ),
        long_case(
            "KO_ENTITY_REPOSITORY_CODEBASE",
            "한결 저장소를 확인해",
            9,
            "그 코드베이스를 검토해",
            "한결 저장소",
            "OntologyEntityReference",
        ),
        long_case(
            "KO_ENTITY_CACHE_STORAGE_LAYER",
            "새롬 캐시를 확인해",
            7,
            "그 저장 계층을 검토해",
            "새롬 캐시",
            "OntologyEntityReference",
        ),
        long_case(
            "KO_ENTITY_FOLDER_DIRECTORY",
            "가온 폴더를 확인해",
            8,
            "그 디렉터리를 검토해",
            "가온 폴더",
            "OntologyEntityReference",
        ),
        long_case(
            "CROSS_EN_KO_ENTITY_SYSTEM",
            "inspect the Solace service",
            8,
            "그 애플리케이션을 검토해",
            "Solace service",
            "OntologyEntityReference",
        ),
        long_case(
            "CROSS_KO_EN_ENTITY_DOCUMENT",
            "마루 보고서를 확인해",
            8,
            "review that document",
            "마루 보고서",
            "OntologyEntityReference",
        ),
        long_case(
            "CROSS_EN_KO_ENTITY_REPOSITORY",
            "inspect the Verdant repository",
            8,
            "그 코드베이스를 검토해",
            "Verdant repository",
            "OntologyEntityReference",
        ),
        long_case(
            "CROSS_KO_EN_ENTITY_FOLDER",
            "보라 폴더를 확인해",
            8,
            "review that directory",
            "보라 폴더",
            "OntologyEntityReference",
        ),
        Case {
            case_id: "EN_ENTITY_HIERARCHY_SELECTS_REPORT",
            turns: vec![
                "inspect the Polar report".into(),
                "inspect the Quartz folder".into(),
                "review that document".into(),
            ],
            expectation: Expectation::Resolve {
                target: "Polar report",
                binding: "OntologyEntityReference",
            },
        },
        Case {
            case_id: "KO_ENTITY_HIERARCHY_SELECTS_REPORT",
            turns: vec![
                "여명 보고서를 확인해".into(),
                "해든 폴더를 확인해".into(),
                "그 문서를 검토해".into(),
            ],
            expectation: Expectation::Resolve {
                target: "여명 보고서",
                binding: "OntologyEntityReference",
            },
        },
        Case {
            case_id: "EN_ENTITY_HIERARCHY_SELECTS_REPOSITORY",
            turns: vec![
                "inspect the Ripple repository".into(),
                "inspect the Tundra file".into(),
                "review that codebase".into(),
            ],
            expectation: Expectation::Resolve {
                target: "Ripple repository",
                binding: "OntologyEntityReference",
            },
        },
        Case {
            case_id: "KO_ENTITY_HIERARCHY_SELECTS_CACHE",
            turns: vec![
                "이음 캐시를 확인해".into(),
                "자람 파일을 확인해".into(),
                "그 저장 계층을 검토해".into(),
            ],
            expectation: Expectation::Resolve {
                target: "이음 캐시",
                binding: "OntologyEntityReference",
            },
        },
        Case {
            case_id: "EN_ENTITY_AMBIGUOUS_DOCUMENT",
            turns: vec![
                "inspect the Umber report".into(),
                "inspect the Willow manual".into(),
                "review that document".into(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "KO_ENTITY_AMBIGUOUS_SYSTEM",
            turns: vec![
                "가람 서비스를 확인해".into(),
                "나래 애플리케이션을 확인해".into(),
                "그 앱을 검토해".into(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "EN_ENTITY_INCOMPATIBLE_ONLY_PERSON",
            turns: vec![
                "Aster says that the build is ready".into(),
                "review that application".into(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "KO_ENTITY_UNBOUND_CODEBASE",
            turns: vec!["그 코드베이스를 검토해".into()],
            expectation: Expectation::Clarify,
        },
        long_case(
            "EN_EVENT_REPAIR_FIX",
            "repair the Zephyr parser",
            7,
            "explain that fix",
            "Zephyr parser",
            "OntologyEventReference",
        ),
        long_case(
            "KO_EVENT_REPAIR_FIX",
            "은하 파서를 수정해",
            7,
            "그 수리를 설명해",
            "은하 파서",
            "OntologyEventReference",
        ),
        long_case(
            "EN_EVENT_DEPLOY_ROLLOUT",
            "deploy the Quill service",
            8,
            "explain that rollout",
            "Quill service",
            "OntologyEventReference",
        ),
        long_case(
            "KO_EVENT_DEPLOY_RELEASE",
            "새봄 서비스를 배포해",
            8,
            "그 출시를 설명해",
            "새봄 서비스",
            "OntologyEventReference",
        ),
        long_case(
            "EN_EVENT_MOVE_TRANSFER",
            "move the Umbral archive",
            9,
            "explain that transfer",
            "Umbral archive",
            "OntologyEventReference",
        ),
        long_case(
            "KO_EVENT_MOVE_TRANSFER",
            "다솜 보관함을 옮겨",
            9,
            "그 이동을 설명해",
            "다솜 보관함",
            "OntologyEventReference",
        ),
        long_case(
            "EN_EVENT_INSPECT_REVIEW",
            "inspect the Velvet repository",
            7,
            "explain that review",
            "Velvet repository",
            "OntologyEventReference",
        ),
        long_case(
            "KO_EVENT_INSPECT_REVIEW",
            "라온 저장소를 확인해",
            7,
            "그 검토를 설명해",
            "라온 저장소",
            "OntologyEventReference",
        ),
        long_case(
            "EN_EVENT_CREATE_DRAFT",
            "create the Wisteria report",
            8,
            "explain that drafting",
            "Wisteria report",
            "OntologyEventReference",
        ),
        long_case(
            "KO_EVENT_CREATE_DRAFT",
            "마음 보고서를 작성해",
            8,
            "그 초안 작성을 설명해",
            "마음 보고서",
            "OntologyEventReference",
        ),
        long_case(
            "CROSS_EN_KO_EVENT_REPAIR",
            "repair the Xenon parser",
            8,
            "그 수리를 설명해",
            "Xenon parser",
            "OntologyEventReference",
        ),
        long_case(
            "CROSS_KO_EN_EVENT_DEPLOY",
            "바다 서비스를 배포해",
            8,
            "explain that rollout",
            "바다 서비스",
            "OntologyEventReference",
        ),
        long_case(
            "CROSS_EN_KO_EVENT_MOVE",
            "move the Yonder archive",
            8,
            "그 이동을 설명해",
            "Yonder archive",
            "OntologyEventReference",
        ),
        long_case(
            "CROSS_KO_EN_EVENT_REVIEW",
            "소담 저장소를 확인해",
            8,
            "explain that review",
            "소담 저장소",
            "OntologyEventReference",
        ),
        Case {
            case_id: "EN_EVENT_ROLE_SELECTS_PARSER_FIX",
            turns: vec![
                "repair the Amber parser".into(),
                "repair the Bronze cache".into(),
                "explain that parser fix".into(),
            ],
            expectation: Expectation::Resolve {
                target: "Amber parser",
                binding: "OntologyEventReference",
            },
        },
        Case {
            case_id: "KO_EVENT_ROLE_SELECTS_SERVER_DEPLOY",
            turns: vec![
                "푸른 서버를 배포해".into(),
                "붉은 서비스를 배포해".into(),
                "그 서버 출시를 설명해".into(),
            ],
            expectation: Expectation::Resolve {
                target: "푸른 서버",
                binding: "OntologyEventReference",
            },
        },
        Case {
            case_id: "EN_EVENT_AMBIGUOUS_FIX",
            turns: vec![
                "repair the Copper parser".into(),
                "repair the Delta cache".into(),
                "explain that fix".into(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "KO_EVENT_AMBIGUOUS_REVIEW",
            turns: vec![
                "여울 저장소를 확인해".into(),
                "시온 보고서를 검토해".into(),
                "그 검토를 설명해".into(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "EN_EVENT_ONTOLOGY_MISMATCH",
            turns: vec![
                "deploy the Ember service".into(),
                "explain that repair".into(),
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "KO_EVENT_NEGATED_NOT_STORED",
            turns: vec!["파서를 수정하지 마".into(), "그 수리를 설명해".into()],
            expectation: Expectation::Clarify,
        },
        Case {
            case_id: "EN_EVENT_QUOTED_REFERENCE",
            turns: vec![
                "repair the Fjord parser".into(),
                "quote ‘that fix failed’".into(),
            ],
            expectation: Expectation::Unchanged {
                text: "quote ‘that fix failed’",
            },
        },
        Case {
            case_id: "KO_ENTITY_QUOTED_REFERENCE",
            turns: vec![
                "하람 서비스를 확인해".into(),
                "‘그 애플리케이션이 느리다’라고 인용해".into(),
            ],
            expectation: Expectation::Unchanged {
                text: "‘그 애플리케이션이 느리다’라고 인용해",
            },
        },
    ]
}

fn main() {
    let mut rows = Vec::new();
    for case in cases() {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut final_response = None;
        for (index, text) in case.turns.iter().enumerate() {
            final_response = Some(
                api.process_conversation_turn(&request(
                    case.case_id,
                    u64::try_from(index + 1).expect("turn index"),
                    text,
                ))
                .expect("conversation turn"),
            );
        }
        let response = final_response.expect("non-empty case");
        let binding_kinds = response
            .reference_resolution
            .discourse_bindings
            .iter()
            .map(|binding| format!("{:?}", binding.kind))
            .collect::<Vec<_>>();
        let ontology_evidence = response
            .reference_resolution
            .discourse_bindings
            .iter()
            .flat_map(|binding| {
                serde_json::to_value(binding)
                    .ok()
                    .and_then(|value| value.get("evidence").cloned())
                    .and_then(|value| serde_json::from_value::<Vec<String>>(value).ok())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        let pass = match case.expectation {
            Expectation::Resolve { target, binding } => {
                response.disposition == ConversationTurnDispositionIR::Grounded
                    && response
                        .reference_resolution
                        .resolved_semantic_text
                        .to_lowercase()
                        .contains(&target.to_lowercase())
                    && binding_kinds.iter().any(|kind| kind == binding)
                    && ontology_evidence
                        .iter()
                        .any(|evidence| evidence.starts_with("ONTOLOGY_PATH:"))
            }
            Expectation::Clarify => {
                response.disposition == ConversationTurnDispositionIR::ClarificationRequired
                    && !response
                        .reference_resolution
                        .ambiguous_reference_surfaces
                        .is_empty()
            }
            Expectation::Unchanged { text } => {
                response.reference_resolution.resolved_semantic_text == text
                    && binding_kinds.iter().all(|kind| {
                        kind != "OntologyEntityReference" && kind != "OntologyEventReference"
                    })
            }
        };
        rows.push(Row {
            case_id: case.case_id.to_string(),
            turn_count: case.turns.len(),
            disposition: response.disposition,
            resolved_semantic_text: response.reference_resolution.resolved_semantic_text,
            binding_kinds,
            ontology_evidence,
            ambiguous_surfaces: response.reference_resolution.ambiguous_reference_surfaces,
            pass,
        });
    }
    println!("{}", serde_json::to_string(&rows).expect("serialize rows"));
    if rows.iter().any(|row| !row.pass) {
        std::process::exit(1);
    }
}
