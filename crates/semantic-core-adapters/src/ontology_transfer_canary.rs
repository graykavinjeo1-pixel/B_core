//! Frozen R14-RUN-0002 transfer suite.
//!
//! These cases use unseen mention directions, event nominalizations, and
//! ambiguity/distance attacks. They are intentionally separate from the
//! implementation-driving R14-RUN-0001 suite.

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
    Unchanged(&'static str),
}

struct Case {
    id: &'static str,
    turns: Vec<&'static str>,
    expectation: Expectation,
}

#[derive(Serialize)]
struct Row {
    id: String,
    resolved_text: String,
    disposition: ConversationTurnDispositionIR,
    binding_kinds: Vec<String>,
    evidence: Vec<String>,
    ambiguous: Vec<String>,
    pass: bool,
}

fn request(id: &str, turn: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
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

fn cases() -> Vec<Case> {
    vec![
        resolve(
            "EN_BACKEND_APP",
            "inspect the Celadon backend",
            "explain that app",
            "Celadon backend",
            "OntologyEntityReference",
        ),
        resolve(
            "EN_DAEMON_APPLICATION",
            "inspect the Dusk daemon",
            "review that application",
            "Dusk daemon",
            "OntologyEntityReference",
        ),
        resolve(
            "EN_MANUAL_DOCUMENT",
            "inspect the Ember manual",
            "review that document",
            "Ember manual",
            "OntologyEntityReference",
        ),
        resolve(
            "EN_CODEBASE_REPOSITORY",
            "inspect the Fable codebase",
            "review that repository",
            "Fable codebase",
            "OntologyEntityReference",
        ),
        resolve(
            "EN_DIRECTORY_FOLDER",
            "inspect the Grove directory",
            "review that folder",
            "Grove directory",
            "OntologyEntityReference",
        ),
        resolve(
            "EN_STORAGE_CACHE",
            "inspect the Harbor storage layer",
            "review that cache",
            "Harbor storage layer",
            "OntologyEntityReference",
        ),
        resolve(
            "KO_BACKEND_APP",
            "이든 백엔드를 확인해",
            "그 앱을 설명해",
            "이든 백엔드",
            "OntologyEntityReference",
        ),
        resolve(
            "KO_DAEMON_APPLICATION",
            "지음 데몬을 확인해",
            "그 애플리케이션을 검토해",
            "지음 데몬",
            "OntologyEntityReference",
        ),
        resolve(
            "KO_MANUAL_DOCUMENT",
            "키움 매뉴얼을 확인해",
            "그 문서를 검토해",
            "키움 매뉴얼",
            "OntologyEntityReference",
        ),
        resolve(
            "KO_CODEBASE_REPOSITORY",
            "누림 코드베이스를 확인해",
            "그 저장소를 검토해",
            "누림 코드베이스",
            "OntologyEntityReference",
        ),
        resolve(
            "KO_DIRECTORY_FOLDER",
            "다림 디렉터리를 확인해",
            "그 폴더를 검토해",
            "다림 디렉터리",
            "OntologyEntityReference",
        ),
        resolve(
            "KO_STORAGE_CACHE",
            "라움 저장 계층을 확인해",
            "그 캐시를 검토해",
            "라움 저장 계층",
            "OntologyEntityReference",
        ),
        resolve(
            "EN_REPAIR_CORRECTION",
            "repair the Mica parser",
            "explain that correction",
            "Mica parser",
            "OntologyEventReference",
        ),
        resolve(
            "EN_DEPLOY_RELEASE",
            "deploy the North server",
            "explain that release",
            "North server",
            "OntologyEventReference",
        ),
        resolve(
            "EN_MOVE_RELOCATION",
            "move the Opal archive",
            "explain that relocation",
            "Opal archive",
            "OntologyEventReference",
        ),
        resolve(
            "EN_INSPECT_EXAMINATION",
            "inspect the Pine repository",
            "explain that examination",
            "Pine repository",
            "OntologyEventReference",
        ),
        resolve(
            "EN_CREATE_AUTHORSHIP",
            "create the Quartz report",
            "explain that authorship",
            "Quartz report",
            "OntologyEventReference",
        ),
        resolve(
            "EN_DELETE_REMOVAL",
            "delete the Rill file",
            "explain that removal",
            "Rill file",
            "OntologyEventReference",
        ),
        resolve(
            "CROSS_REPAIR_KO",
            "repair the Slate parser",
            "그 수리를 설명해",
            "Slate parser",
            "OntologyEventReference",
        ),
        resolve(
            "CROSS_DEPLOY_EN",
            "토담 서버를 배포해",
            "explain that deployment",
            "토담 서버",
            "OntologyEventReference",
        ),
        Case {
            id: "AMBIGUOUS_TWO_APPLICATIONS",
            turns: vec![
                "inspect the Umber backend",
                "inspect the Vale daemon",
                "review that application",
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            id: "AMBIGUOUS_TWO_DOCUMENTS",
            turns: vec![
                "inspect the Wren manual",
                "inspect the Xylem report",
                "review that document",
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            id: "EVENT_MISMATCH",
            turns: vec!["repair the Yarrow parser", "explain that rollout"],
            expectation: Expectation::Clarify,
        },
        Case {
            id: "EVENT_ROLE_AMBIGUOUS",
            turns: vec![
                "repair the Zinnia parser",
                "fix the Aster parser",
                "explain that parser correction",
            ],
            expectation: Expectation::Clarify,
        },
        Case {
            id: "QUOTED_ENTITY_UNCHANGED",
            turns: vec!["inspect the Birch backend", "quote ‘that app is slow’"],
            expectation: Expectation::Unchanged("quote ‘that app is slow’"),
        },
        Case {
            id: "OUT_OF_WINDOW_ENTITY",
            turns: {
                let mut turns = vec!["inspect the Cedar daemon"];
                turns.extend(std::iter::repeat_n("review the plan", 17));
                turns.push("review that application");
                turns
            },
            expectation: Expectation::Clarify,
        },
    ]
}

fn resolve(
    id: &'static str,
    antecedent: &'static str,
    reference: &'static str,
    target: &'static str,
    binding: &'static str,
) -> Case {
    Case {
        id,
        turns: vec![antecedent, "review the plan", "inspect the file", reference],
        expectation: Expectation::Resolve { target, binding },
    }
}

fn main() {
    let rows = cases()
        .into_iter()
        .map(|case| {
            let mut api = CognitiveApi::new_embedded().expect("embedded core");
            let response = case
                .turns
                .iter()
                .enumerate()
                .map(|(index, text)| {
                    api.process_conversation_turn(&request(
                        case.id,
                        u64::try_from(index + 1).expect("turn index"),
                        text,
                    ))
                    .expect("conversation turn")
                })
                .last()
                .expect("non-empty case");
            let bindings = &response.reference_resolution.discourse_bindings;
            let binding_kinds = bindings
                .iter()
                .map(|binding| format!("{:?}", binding.kind))
                .collect::<Vec<_>>();
            let evidence = bindings
                .iter()
                .flat_map(|binding| binding.evidence.iter().cloned())
                .collect::<Vec<_>>();
            let resolved_text = response.reference_resolution.resolved_semantic_text;
            let ambiguous = response.reference_resolution.ambiguous_reference_surfaces;
            let pass = match case.expectation {
                Expectation::Resolve { target, binding } => {
                    response.disposition == ConversationTurnDispositionIR::Grounded
                        && resolved_text
                            .to_lowercase()
                            .contains(&target.to_lowercase())
                        && binding_kinds.iter().any(|kind| kind == binding)
                        && evidence
                            .iter()
                            .any(|item| item.starts_with("ONTOLOGY_PATH:"))
                        && evidence
                            .iter()
                            .any(|item| item == "SEMANTIC_AUTHORITY:false")
                }
                Expectation::Clarify => {
                    response.disposition == ConversationTurnDispositionIR::ClarificationRequired
                        && !ambiguous.is_empty()
                        && binding_kinds
                            .iter()
                            .all(|kind| !kind.starts_with("Ontology"))
                }
                Expectation::Unchanged(expected) => {
                    resolved_text == expected
                        && binding_kinds
                            .iter()
                            .all(|kind| !kind.starts_with("Ontology"))
                }
            };
            Row {
                id: case.id.to_string(),
                resolved_text,
                disposition: response.disposition,
                binding_kinds,
                evidence,
                ambiguous,
                pass,
            }
        })
        .collect::<Vec<_>>();
    println!("{}", serde_json::to_string(&rows).expect("serialize rows"));
    if rows.iter().any(|row| !row.pass) {
        std::process::exit(1);
    }
}
