//! Frozen R41-RUN-0001 diagnostic suite.
//!
//! These cases require clause-local pragmatic force, typed relations, and a
//! fail-closed selection decision. The suite is intentionally public-API only.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

const COMPOSITION_SCHEMA: &str = "B_CORE_COMPOSITIONAL_PRAGMATIC_GRAPH_IR_1";

#[derive(Clone, Copy)]
struct Case {
    id: &'static str,
    category: &'static str,
    setup: Option<&'static str>,
    text: &'static str,
    language: LanguageCodeIR,
    expected: Option<&'static str>,
    forbidden: Option<&'static str>,
    relation: Option<&'static str>,
    clarification: bool,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
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
        max_plan_steps: 12,
    }
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    if let Some(setup) = case.setup {
        api.process_conversation_turn(&request(case.id, 1, setup, case.language))
            .expect("setup turn");
    }
    let turn = u64::from(case.setup.is_some()) + 1;
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.text, case.language))
        .expect("R41 diagnostic turn");
    let structure = serde_json::to_string(&response.pragmatic_interpretation)
        .expect("pragmatic json")
        .to_uppercase();
    let authorized = response
        .conversation_state
        .active_goals
        .iter()
        .filter(|goal| goal.external_execution_authorized)
        .collect::<Vec<_>>();
    let expected_ok = case.expected.map_or_else(
        || authorized.is_empty(),
        |predicate| {
            authorized
                .iter()
                .any(|goal| goal.canonical_predicate.eq_ignore_ascii_case(predicate))
                || (case.relation == Some("CONDITIONS")
                    && response
                        .conversation_state
                        .deferred_action_commitments
                        .iter()
                        .any(|commitment| {
                            commitment
                                .action
                                .canonical_predicate
                                .eq_ignore_ascii_case(predicate)
                                && commitment
                                    .action
                                    .external_execution_authorized_after_verification
                        }))
        },
    );
    let forbidden_ok = case.forbidden.is_none_or(|predicate| {
        authorized
            .iter()
            .all(|goal| !goal.canonical_predicate.eq_ignore_ascii_case(predicate))
    });
    let relation_ok = case
        .relation
        .is_none_or(|relation| structure.contains(relation));
    let clarification_ok = response
        .pragmatic_interpretation
        .compositional_analysis
        .clarification_required
        == case.clarification;
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: structure.contains(COMPOSITION_SCHEMA)
            && structure.contains("GRAPH_SHA256")
            && structure.contains("SEMANTIC_AUTHORITY\":FALSE")
            && structure.contains("EXTERNAL_ACTION_EXECUTION_AUTHORIZED\":FALSE")
            && expected_ok
            && forbidden_ok
            && relation_ok
            && clarification_ok
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![
            format!("authorized={authorized:?}"),
            format!(
                "deferred={:?}",
                response.conversation_state.deferred_action_commitments
            ),
            format!(
                "clarification={}",
                response
                    .pragmatic_interpretation
                    .compositional_analysis
                    .clarification_required
            ),
            structure,
            response.output.text,
        ],
    }
}

fn cases() -> Vec<Case> {
    use LanguageCodeIR::{English, Korean};
    vec![
        Case {
            id: "R41_REPORT_01",
            category: "reported_vs_request",
            setup: None,
            text: "민수는 캐시를 삭제하자고 말했지만, 로그를 검사해줄래?",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            relation: Some("CONTRASTS"),
            clarification: false,
        },
        Case {
            id: "R41_REPORT_02",
            category: "reported_vs_request",
            setup: None,
            text: "Dana said to delete the cache, but would you inspect the log?",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            relation: Some("CONTRASTS"),
            clarification: false,
        },
        Case {
            id: "R41_REPORT_03",
            category: "reported_vs_request",
            setup: None,
            text: "팀은 워커를 수리하자고 했지만 큐를 확인해줬으면 해",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("REPAIR"),
            relation: Some("CONTRASTS"),
            clarification: false,
        },
        Case {
            id: "R41_REPORT_04",
            category: "reported_vs_request",
            setup: None,
            text: "The team suggested repairing the worker; instead, inspect the queue for me.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("REPAIR"),
            relation: Some("OVERRIDES"),
            clarification: false,
        },
        Case {
            id: "R41_PROHIBIT_01",
            category: "prohibition_alternative",
            setup: None,
            text: "캐시는 삭제하지 말고 로그를 검사해줘",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            relation: Some("PROHIBITS"),
            clarification: false,
        },
        Case {
            id: "R41_PROHIBIT_02",
            category: "prohibition_alternative",
            setup: None,
            text: "Do not delete the cache; inspect the log instead.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            relation: Some("PROHIBITS"),
            clarification: false,
        },
        Case {
            id: "R41_PROHIBIT_03",
            category: "prohibition_alternative",
            setup: None,
            text: "워커를 수리하지 말고 큐를 분석해줘",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("REPAIR"),
            relation: Some("PROHIBITS"),
            clarification: false,
        },
        Case {
            id: "R41_PROHIBIT_04",
            category: "prohibition_alternative",
            setup: None,
            text: "Don't repair the worker; verify the queue instead.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("REPAIR"),
            relation: Some("PROHIBITS"),
            clarification: false,
        },
        Case {
            id: "R41_EVAL_01",
            category: "evaluation_then_request",
            setup: None,
            text: "이걸 완료라고 부르겠어? 로그를 확인해줄래?",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            relation: Some("SEQUENCES"),
            clarification: false,
        },
        Case {
            id: "R41_EVAL_02",
            category: "evaluation_then_request",
            setup: None,
            text: "Who would call this fixed? Please inspect the log.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            relation: Some("SEQUENCES"),
            clarification: false,
        },
        Case {
            id: "R41_EVAL_03",
            category: "evaluation_then_request",
            setup: None,
            text: "이게 성공이라고? 워커를 분석해줘",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            relation: Some("SEQUENCES"),
            clarification: false,
        },
        Case {
            id: "R41_EVAL_04",
            category: "evaluation_then_request",
            setup: None,
            text: "You call that repaired? Would you verify the worker now?",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            relation: Some("SEQUENCES"),
            clarification: false,
        },
        Case {
            id: "R41_REASON_01",
            category: "reason_condition_scope",
            setup: None,
            text: "워커가 실패했기 때문에 큐를 검사해줄래?",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            relation: Some("SUPPORTS"),
            clarification: false,
        },
        Case {
            id: "R41_REASON_02",
            category: "reason_condition_scope",
            setup: None,
            text: "Because the worker failed, would you inspect the queue?",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            relation: Some("SUPPORTS"),
            clarification: false,
        },
        Case {
            id: "R41_REASON_03",
            category: "reason_condition_scope",
            setup: None,
            text: "캐시가 깨지면 로그를 분석해줘",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            relation: Some("CONDITIONS"),
            clarification: false,
        },
        Case {
            id: "R41_REASON_04",
            category: "reason_condition_scope",
            setup: None,
            text: "If the cache fails, inspect the log.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: None,
            relation: Some("CONDITIONS"),
            clarification: false,
        },
        Case {
            id: "R41_CAPABILITY_01",
            category: "capability_not_request",
            setup: None,
            text: "네가 큐를 검사할 수 있는지 알려줘",
            language: Korean,
            expected: None,
            forbidden: Some("INVESTIGATE"),
            relation: None,
            clarification: false,
        },
        Case {
            id: "R41_CAPABILITY_02",
            category: "capability_not_request",
            setup: None,
            text: "Are you able to inspect the queue?",
            language: English,
            expected: None,
            forbidden: Some("INVESTIGATE"),
            relation: None,
            clarification: false,
        },
        Case {
            id: "R41_CAPABILITY_03",
            category: "capability_not_request",
            setup: None,
            text: "로그를 분석할 능력이 있어?",
            language: Korean,
            expected: None,
            forbidden: Some("INVESTIGATE"),
            relation: None,
            clarification: false,
        },
        Case {
            id: "R41_CAPABILITY_04",
            category: "capability_not_request",
            setup: None,
            text: "Can the system repair the worker?",
            language: English,
            expected: None,
            forbidden: Some("REPAIR"),
            relation: None,
            clarification: false,
        },
        Case {
            id: "R41_AMBIG_01",
            category: "ambiguous_alternative",
            setup: None,
            text: "큐를 검사하거나 워커를 수리해줘",
            language: Korean,
            expected: None,
            forbidden: None,
            relation: Some("ALTERNATIVE"),
            clarification: true,
        },
        Case {
            id: "R41_AMBIG_02",
            category: "ambiguous_alternative",
            setup: None,
            text: "Inspect the queue or repair the worker.",
            language: English,
            expected: None,
            forbidden: None,
            relation: Some("ALTERNATIVE"),
            clarification: true,
        },
        Case {
            id: "R41_AMBIG_03",
            category: "ambiguous_alternative",
            setup: None,
            text: "캐시를 삭제할지 로그를 분석할지 해줘",
            language: Korean,
            expected: None,
            forbidden: None,
            relation: Some("ALTERNATIVE"),
            clarification: true,
        },
        Case {
            id: "R41_AMBIG_04",
            category: "ambiguous_alternative",
            setup: None,
            text: "Either delete the cache or inspect the log.",
            language: English,
            expected: None,
            forbidden: None,
            relation: Some("ALTERNATIVE"),
            clarification: true,
        },
        Case {
            id: "R41_CORRECT_01",
            category: "multi_turn_correction",
            setup: Some("파서를 수리해줘"),
            text: "아니, 파서를 수리하지 말고 검사해줘",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("REPAIR"),
            relation: Some("CORRECTS"),
            clarification: false,
        },
        Case {
            id: "R41_CORRECT_02",
            category: "multi_turn_correction",
            setup: Some("Repair the parser."),
            text: "No, do not repair it; inspect it instead.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("REPAIR"),
            relation: Some("CORRECTS"),
            clarification: false,
        },
        Case {
            id: "R41_CORRECT_03",
            category: "multi_turn_correction",
            setup: Some("워커를 삭제해줘"),
            text: "아니, 삭제하지 말고 워커를 분석해줘",
            language: Korean,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            relation: Some("CORRECTS"),
            clarification: false,
        },
        Case {
            id: "R41_CORRECT_04",
            category: "multi_turn_correction",
            setup: Some("Delete the cache."),
            text: "Actually, don't delete it; inspect the cache instead.",
            language: English,
            expected: Some("INVESTIGATE"),
            forbidden: Some("DELETE"),
            relation: Some("CORRECTS"),
            clarification: false,
        },
    ]
}

fn main() {
    let rows = cases().into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    println!("{}", serde_json::to_string_pretty(&rows).expect("rows"));
    println!("R41_DIAGNOSTIC_PASSED={passed}/{}", rows.len());
    if passed != rows.len() {
        std::process::exit(1);
    }
}
