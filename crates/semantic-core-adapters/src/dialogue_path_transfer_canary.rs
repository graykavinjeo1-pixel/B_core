//! Frozen R16-RUN-0002 held-out multi-hop transfer and attack suite.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    DialogueRelationAnswerDispositionIR, DialogueRelationKindIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA, MAX_DIALOGUE_RELATION_PATH_HOPS,
};
use serde::Serialize;

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    edges: usize,
    paths: usize,
    hops: Vec<usize>,
    disposition: Option<DialogueRelationAnswerDispositionIR>,
    truncated: bool,
    pass: bool,
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

fn run_turns(
    id: &str,
    turns: &[(String, LanguageCodeIR)],
) -> (CognitiveApi, semantic_core_adapters::ConversationStateIR) {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut state = None;
    for (index, (text, language)) in turns.iter().enumerate() {
        state = Some(
            api.process_conversation_turn(&request(
                id,
                u64::try_from(index + 1).expect("bounded turn"),
                text,
                *language,
            ))
            .expect("turn")
            .conversation_state,
        );
    }
    (api, state.expect("non-empty turns"))
}

fn answer_row(
    id: &str,
    category: &str,
    mut api: CognitiveApi,
    state: semantic_core_adapters::ConversationStateIR,
    query: &str,
    language: LanguageCodeIR,
    predicate: impl FnOnce(&semantic_core_adapters::DialogueRelationAnswerIR) -> bool,
) -> Row {
    let response = api
        .process_conversation_turn(&request(id, state.completed_turns + 1, query, language))
        .expect("query");
    let answer = response.dialogue_relation_answer.as_ref();
    let paths = answer.map_or(0, |answer| answer.paths.len());
    let hops = answer.map_or_else(Vec::new, |answer| {
        answer.paths.iter().map(|path| path.hop_count).collect()
    });
    let truncated =
        answer.is_some_and(|answer| answer.paths.iter().any(|path| path.truncated_by_hop_limit));
    let disposition = answer.map(|answer| answer.disposition);
    let pass = state
        .dialogue_relation_graph
        .validate_with_ledger(state.completed_turns, &state.epistemic_ledger)
        && answer.is_some_and(predicate)
        && response.grounded_response.is_none()
        && response.output.grounded_plan_sha256.is_none();
    Row {
        id: id.to_string(),
        category: category.to_string(),
        edges: state.dialogue_relation_graph.relations.len(),
        paths,
        hops,
        disposition,
        truncated,
        pass,
    }
}

fn variant_chain(id: &str, name: &str, connectors: [&str; 3], cross: bool) -> Row {
    let languages = if cross {
        [
            LanguageCodeIR::English,
            LanguageCodeIR::Korean,
            LanguageCodeIR::English,
            LanguageCodeIR::Korean,
        ]
    } else {
        [LanguageCodeIR::English; 4]
    };
    let turns = vec![
        (format!("{name} origin fault"), languages[0]),
        (
            format!("{}, {name} relay pressure", connectors[0]),
            languages[1],
        ),
        (
            format!("{}, {name} worker saturation", connectors[1]),
            languages[2],
        ),
        (
            format!("{}, {name} response decline", connectors[2]),
            languages[3],
        ),
    ];
    let (api, state) = run_turns(id, &turns);
    answer_row(
        id,
        "unseen_marker_composition",
        api,
        state,
        &format!("Why {name} response decline?"),
        LanguageCodeIR::English,
        |answer| {
            answer.disposition == DialogueRelationAnswerDispositionIR::AnsweredFromDialoguePath
                && answer.paths.len() == 1
                && answer.paths[0].hop_count == 3
                && !answer.paths[0].truncated_by_hop_limit
        },
    )
}

fn branch_case(id: &str, name: &str) -> Row {
    let turns = vec![
        (format!("{name} cache pressure"), LanguageCodeIR::English),
        (
            format!("Because of that, {name} latency increase"),
            LanguageCodeIR::English,
        ),
        (
            format!("{name} network congestion"),
            LanguageCodeIR::English,
        ),
        (
            format!("For that reason, {name} latency increase"),
            LanguageCodeIR::English,
        ),
    ];
    let (api, state) = run_turns(id, &turns);
    answer_row(
        id,
        "multiple_independent_paths",
        api,
        state,
        &format!("Why {name} latency increase?"),
        LanguageCodeIR::English,
        |answer| {
            answer.disposition == DialogueRelationAnswerDispositionIR::MultipleDialogueRelations
                && answer.paths.len() == 2
                && answer.paths.iter().all(|path| path.hop_count == 1)
        },
    )
}

fn long_chain(id: &str, name: &str) -> Row {
    let mut turns = vec![(format!("{name} stage zero state"), LanguageCodeIR::English)];
    for index in 1..=8 {
        turns.push((
            format!("Therefore, {name} stage {index} state"),
            LanguageCodeIR::English,
        ));
    }
    let (api, state) = run_turns(id, &turns);
    answer_row(
        id,
        "bounded_long_path",
        api,
        state,
        &format!("Why {name} stage 8 state?"),
        LanguageCodeIR::English,
        |answer| {
            answer.disposition == DialogueRelationAnswerDispositionIR::AnsweredFromDialoguePath
                && answer.paths.len() == 1
                && answer.paths[0].hop_count == MAX_DIALOGUE_RELATION_PATH_HOPS
                && answer.paths[0].truncated_by_hop_limit
        },
    )
}

fn concession_isolation(id: &str, name: &str, korean: bool) -> Row {
    let turns = if korean {
        vec![
            (format!("{name} 전환 고비용"), LanguageCodeIR::Korean),
            (
                format!("그럼에도, {name} 배포 계속"),
                LanguageCodeIR::Korean,
            ),
            (
                format!("따라서, {name} 준비도 증가"),
                LanguageCodeIR::Korean,
            ),
        ]
    } else {
        vec![
            (
                format!("{name} migration high cost"),
                LanguageCodeIR::English,
            ),
            (
                format!("Even so, {name} rollout continued"),
                LanguageCodeIR::English,
            ),
            (
                format!("Therefore, {name} readiness growth"),
                LanguageCodeIR::English,
            ),
        ]
    };
    let (api, state) = run_turns(id, &turns);
    let query = if korean {
        format!("왜 {name} 준비도 증가?")
    } else {
        format!("Why {name} readiness growth?")
    };
    answer_row(
        id,
        "concession_not_causal",
        api,
        state,
        &query,
        if korean {
            LanguageCodeIR::Korean
        } else {
            LanguageCodeIR::English
        },
        |answer| {
            answer.disposition == DialogueRelationAnswerDispositionIR::AnsweredFromDialogueRelation
                && answer.paths.len() == 1
                && answer.paths[0].hop_count == 1
                && answer
                    .evidence
                    .iter()
                    .all(|edge| edge.kind != DialogueRelationKindIR::Concession)
        },
    )
}

fn modal_case(id: &str, name: &str) -> Row {
    let turns = vec![
        (
            format!("Perhaps {name} storage fault"),
            LanguageCodeIR::English,
        ),
        (
            format!("As a result, {name} retry growth"),
            LanguageCodeIR::English,
        ),
        (
            format!("Consequently, {name} throughput decline"),
            LanguageCodeIR::English,
        ),
    ];
    let (api, state) = run_turns(id, &turns);
    answer_row(
        id,
        "modal_world_preservation",
        api,
        state,
        &format!("Why {name} throughput decline?"),
        LanguageCodeIR::English,
        |answer| {
            answer.paths.len() == 1
                && answer.paths[0].hop_count == 2
                && answer.paths[0].contains_nonactual_world
                && answer.realized_text.contains("non-actual")
        },
    )
}

fn retraction_case(id: &str, name: &str, korean: bool) -> Row {
    let turns = if korean {
        vec![
            (format!("{name} 캐시 장애"), LanguageCodeIR::Korean),
            (
                format!("그 때문에, {name} 서비스 지연"),
                LanguageCodeIR::Korean,
            ),
            (
                format!("그래서, {name} 대기열 증가"),
                LanguageCodeIR::Korean,
            ),
        ]
    } else {
        vec![
            (format!("{name} cache fault"), LanguageCodeIR::English),
            (
                format!("Because of that, {name} service latency"),
                LanguageCodeIR::English,
            ),
            (
                format!("Therefore, {name} queue growth"),
                LanguageCodeIR::English,
            ),
        ]
    };
    let (mut api, _) = run_turns(id, &turns);
    let language = if korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let retracted = api
        .process_conversation_turn(&request(
            id,
            4,
            if korean {
                "그 주장을 철회해"
            } else {
                "Retract that claim"
            },
            language,
        ))
        .expect("retraction");
    let query = if korean {
        format!("왜 {name} 대기열 증가?")
    } else {
        format!("Why {name} queue growth?")
    };
    answer_row(
        id,
        "held_out_retraction",
        api,
        retracted.conversation_state,
        &query,
        language,
        |answer| {
            answer.disposition == DialogueRelationAnswerDispositionIR::NoMatchingDialogueRelation
                && answer.paths.is_empty()
                && answer.evidence.is_empty()
        },
    )
}

fn main() {
    let mut rows = vec![
        variant_chain(
            "VARIANT_A",
            "Tundra",
            ["For that reason", "Consequently", "As a result"],
            false,
        ),
        variant_chain(
            "VARIANT_B",
            "Umber",
            ["As a result", "Therefore", "Consequently"],
            false,
        ),
        variant_chain(
            "VARIANT_C",
            "Vale",
            ["Because of that", "As a result", "Therefore"],
            false,
        ),
        variant_chain(
            "VARIANT_X1",
            "Wren",
            ["그 때문에", "Therefore", "그래서"],
            true,
        ),
        variant_chain(
            "VARIANT_X2",
            "Xylem",
            ["그래서", "Consequently", "따라서"],
            true,
        ),
        variant_chain(
            "VARIANT_X3",
            "Yarrow",
            ["그런 이유로", "As a result", "그 결과"],
            true,
        ),
    ];
    for (id, name) in [
        ("BRANCH_1", "Zinnia"),
        ("BRANCH_2", "Aster"),
        ("BRANCH_3", "Beryl"),
        ("BRANCH_4", "Cobalt"),
    ] {
        rows.push(branch_case(id, name));
    }
    rows.push(long_chain("LONG_1", "Dahlia"));
    rows.push(long_chain("LONG_2", "Ecru"));
    rows.push(concession_isolation("CONCESSION_EN", "Flint", false));
    rows.push(concession_isolation("CONCESSION_KO", "누림", true));
    rows.push(modal_case("MODAL_X1", "Garnet"));
    rows.push(modal_case("MODAL_X2", "Helix"));
    rows.push(retraction_case("RETRACT_X1", "Indigo", false));
    rows.push(retraction_case("RETRACT_X2", "Jade", false));
    rows.push(retraction_case("RETRACT_X3", "키움", true));
    rows.push(retraction_case("RETRACT_X4", "도담", true));

    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R16-RUN-0002",
        "held_out_until_after_diagnostic_repairs": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "recursive_source_mutations": 0,
        "total": rows.len(),
        "passed": passed,
        "failed": rows.len() - passed,
        "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&payload).expect("json"));
    if passed != payload["total"].as_u64().unwrap_or_default() as usize {
        std::process::exit(1);
    }
}
