use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CandidateDispositionIR, CognitiveApi, CompositionalSemanticAnalyzer,
    ConversationInputModalityIR, ConversationTurnDispositionIR, ConversationTurnRequestIR,
    DiscourseBindingKindIR, EventRelationKindIR, LanguageCodeIR, QuantifierKindIR,
    SemanticRoleGraphIR, SemanticRoleKindIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

struct RoleCase {
    case_id: &'static str,
    text: &'static str,
    roles: &'static [(SemanticRoleKindIR, &'static str)],
    quantifier: Option<(QuantifierKindIR, Option<u64>)>,
    relation: Option<EventRelationKindIR>,
}

#[derive(Serialize)]
struct Row {
    case_id: String,
    graph_valid: bool,
    assertions: usize,
    satisfied: usize,
    authority_safe: bool,
    pass: bool,
}

fn has_role(graph: &SemanticRoleGraphIR, role: SemanticRoleKindIR, surface: &str) -> bool {
    graph.role_edges.iter().any(|edge| {
        edge.role == role
            && graph
                .nodes
                .iter()
                .any(|node| node.node_id == edge.argument_node_id && node.surface.contains(surface))
    })
}

fn request(conversation_id: &str, turn: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        turn_index: turn,
        request_id: format!("{conversation_id}-{turn}"),
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

fn main() {
    let cases = [
        RoleCase {
            case_id: "KO_AGENT_SOURCE_THEME_ALL",
            text: "사용자가 서버에서 모든 파일을 읽어",
            roles: &[
                (SemanticRoleKindIR::Agent, "사용자"),
                (SemanticRoleKindIR::Source, "서버"),
                (SemanticRoleKindIR::Theme, "모든 파일"),
            ],
            quantifier: Some((QuantifierKindIR::All, None)),
            relation: None,
        },
        RoleCase {
            case_id: "KO_INSTRUMENT_THEME",
            text: "파서로 코드를 수정해",
            roles: &[
                (SemanticRoleKindIR::Instrument, "파서"),
                (SemanticRoleKindIR::Theme, "코드"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "KO_RECIPIENT_THEME",
            text: "관리자에게 보고서를 보내",
            roles: &[
                (SemanticRoleKindIR::Recipient, "관리자"),
                (SemanticRoleKindIR::Theme, "보고서"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "KO_RESULT_THEME",
            text: "json으로 파일을 변환해",
            roles: &[
                (SemanticRoleKindIR::Result, "json"),
                (SemanticRoleKindIR::Theme, "파일"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "KO_COMPARISON_PEER",
            text: "파일과 보고서를 비교해",
            roles: &[
                (SemanticRoleKindIR::ComparisonPeer, "파일"),
                (SemanticRoleKindIR::Theme, "보고서"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "KO_TEMPORAL_PRIOR_RESULT",
            text: "파일을 읽고 변환한 뒤 저장해",
            roles: &[(SemanticRoleKindIR::PriorResult, "변환")],
            quantifier: None,
            relation: Some(EventRelationKindIR::TemporalBefore),
        },
        RoleCase {
            case_id: "KO_CONDITION_RELATION",
            text: "파일을 확인하면 보고서를 저장해",
            roles: &[
                (SemanticRoleKindIR::Theme, "파일"),
                (SemanticRoleKindIR::Theme, "보고서"),
            ],
            quantifier: None,
            relation: Some(EventRelationKindIR::Condition),
        },
        RoleCase {
            case_id: "KO_EXACT_CARDINALITY",
            text: "정확히 3개 파일을 분석해",
            roles: &[(SemanticRoleKindIR::Theme, "파일")],
            quantifier: Some((QuantifierKindIR::Exactly, Some(3))),
            relation: None,
        },
        RoleCase {
            case_id: "KO_NEGATED_NONE_SCOPE",
            text: "아무 파일도 삭제하지 마",
            roles: &[(SemanticRoleKindIR::Theme, "아무 파일")],
            quantifier: Some((QuantifierKindIR::None, None)),
            relation: None,
        },
        RoleCase {
            case_id: "KO_RELATIVE_NOUN_PHRASE",
            text: "사용자가 올린 보고서를 검토해",
            roles: &[
                (SemanticRoleKindIR::Agent, "사용자"),
                (SemanticRoleKindIR::Theme, "올린 보고서"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "EN_PASSIVE_AGENT_PATIENT",
            text: "the report was reviewed by alice with a parser",
            roles: &[
                (SemanticRoleKindIR::Patient, "report"),
                (SemanticRoleKindIR::Agent, "alice"),
                (SemanticRoleKindIR::Instrument, "parser"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "EN_ACTIVE_AGENT_THEME",
            text: "alice reviewed the report",
            roles: &[
                (SemanticRoleKindIR::Agent, "alice"),
                (SemanticRoleKindIR::Theme, "report"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "EN_RECIPIENT_THEME",
            text: "send report to reviewer",
            roles: &[
                (SemanticRoleKindIR::Theme, "report"),
                (SemanticRoleKindIR::Recipient, "reviewer"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "EN_SOURCE_DESTINATION",
            text: "move file from inbox to archive",
            roles: &[
                (SemanticRoleKindIR::Theme, "file"),
                (SemanticRoleKindIR::Source, "inbox"),
                (SemanticRoleKindIR::Destination, "archive"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "EN_INSTRUMENT_THEME",
            text: "repair code with parser",
            roles: &[
                (SemanticRoleKindIR::Theme, "code"),
                (SemanticRoleKindIR::Instrument, "parser"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "EN_COMPARISON_PEER",
            text: "compare file with report",
            roles: &[
                (SemanticRoleKindIR::Theme, "file"),
                (SemanticRoleKindIR::ComparisonPeer, "report"),
            ],
            quantifier: None,
            relation: None,
        },
        RoleCase {
            case_id: "EN_EACH_SCOPE",
            text: "check each file",
            roles: &[(SemanticRoleKindIR::Theme, "each file")],
            quantifier: Some((QuantifierKindIR::Each, None)),
            relation: None,
        },
        RoleCase {
            case_id: "EN_EXACT_CARDINALITY",
            text: "analyze exactly 4 logs",
            roles: &[(SemanticRoleKindIR::Theme, "exactly 4 logs")],
            quantifier: Some((QuantifierKindIR::Exactly, Some(4))),
            relation: None,
        },
        RoleCase {
            case_id: "EN_TEMPORAL_SEQUENCE",
            text: "check logs, then repair code",
            roles: &[
                (SemanticRoleKindIR::Theme, "logs"),
                (SemanticRoleKindIR::Theme, "code"),
            ],
            quantifier: None,
            relation: Some(EventRelationKindIR::TemporalBefore),
        },
        RoleCase {
            case_id: "EN_CAUSE_RELATION",
            text: "analyze logs, so repair code",
            roles: &[
                (SemanticRoleKindIR::Theme, "logs"),
                (SemanticRoleKindIR::Theme, "code"),
            ],
            quantifier: None,
            relation: Some(EventRelationKindIR::Cause),
        },
    ];

    let mut rows = Vec::new();
    for case in cases {
        let analysis = CompositionalSemanticAnalyzer.analyze(case.text);
        let graph = &analysis.semantic_role_graph;
        let mut assertions = case.roles.len();
        let mut satisfied = case
            .roles
            .iter()
            .filter(|(role, surface)| has_role(graph, *role, surface))
            .count();
        if let Some((quantifier, cardinality)) = case.quantifier {
            assertions += 1;
            if graph
                .quantifier_scopes
                .iter()
                .any(|scope| scope.quantifier == quantifier && scope.cardinality == cardinality)
            {
                satisfied += 1;
            }
        }
        if let Some(relation) = case.relation {
            assertions += 1;
            if graph
                .event_relations
                .iter()
                .any(|edge| edge.relation == relation)
            {
                satisfied += 1;
            }
        }
        let authority_safe = analysis
            .candidates
            .iter()
            .filter(|candidate| candidate.disposition != CandidateDispositionIR::Viable)
            .all(|candidate| !candidate.external_execution_authorized);
        let graph_valid = graph.validate();
        rows.push(Row {
            case_id: case.case_id.to_string(),
            graph_valid,
            assertions,
            satisfied,
            authority_safe,
            pass: graph_valid && authority_safe && assertions == satisfied,
        });
    }

    for (case_id, turns, expected_binding, ambiguous) in [
        (
            "KO_EVENT_REFERENCE_AUTHORITY",
            &["보고서를 저장해", "그 작업을 설명해"][..],
            Some(DiscourseBindingKindIR::EventReference),
            false,
        ),
        (
            "EN_RESULT_REFERENCE_PROVENANCE",
            &["save report", "explain that result"][..],
            Some(DiscourseBindingKindIR::ResultReference),
            false,
        ),
        (
            "KO_PROPOSITION_REFERENCE",
            &["빌드가 실패했다", "그 사실을 설명해"][..],
            Some(DiscourseBindingKindIR::PropositionReference),
            false,
        ),
        (
            "EN_PROPOSITION_AMBIGUITY",
            &["the build failed. the log was empty.", "explain that fact"][..],
            None,
            true,
        ),
    ] {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut response = None;
        for (index, text) in turns.iter().enumerate() {
            response = Some(
                api.process_conversation_turn(&request(
                    case_id,
                    u64::try_from(index + 1).expect("turn index"),
                    text,
                ))
                .expect("conversation turn"),
            );
        }
        let response = response.expect("final response");
        let binding = response
            .reference_resolution
            .discourse_bindings
            .first()
            .map(|binding| binding.kind);
        let inner_commands_safe = response
            .pragmatic_interpretation
            .compositional_analysis
            .candidates
            .iter()
            .filter(|candidate| candidate.intent == PlanIntentIR::Execute)
            .all(|candidate| candidate.disposition != CandidateDispositionIR::Viable);
        let disposition_matches = if ambiguous {
            response.disposition == ConversationTurnDispositionIR::ClarificationRequired
        } else {
            response.disposition == ConversationTurnDispositionIR::Grounded
        };
        let pass = binding == expected_binding && disposition_matches && inner_commands_safe;
        rows.push(Row {
            case_id: case_id.to_string(),
            graph_valid: response
                .pragmatic_interpretation
                .compositional_analysis
                .semantic_role_graph
                .validate(),
            assertions: 3,
            satisfied: usize::from(binding == expected_binding)
                + usize::from(disposition_matches)
                + usize::from(inner_commands_safe),
            authority_safe: inner_commands_safe,
            pass,
        });
    }

    println!("{}", serde_json::to_string(&rows).expect("serialize rows"));
    if rows.iter().any(|row| !row.pass) {
        std::process::exit(1);
    }
}
