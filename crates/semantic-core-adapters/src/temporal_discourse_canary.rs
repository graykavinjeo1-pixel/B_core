use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    TemporalAnswerDispositionIR, TemporalGraphIR, TemporalQaEngine, TemporalRelationKindIR,
    TemporalRelationStatusIR, TemporalSemanticAnalyzer, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Serialize)]
struct CanaryRow {
    id: String,
    family: String,
    input: String,
    pass: bool,
    observed: String,
}

#[derive(Serialize)]
struct CanaryReport {
    schema: &'static str,
    status: &'static str,
    total: usize,
    passed: usize,
    failed: usize,
    english_surface: usize,
    korean_surface: usize,
    temporal_expressions: usize,
    temporal_questions: usize,
    cross_turn_and_transitive: usize,
    conflict_and_tamper: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    rows: Vec<CanaryRow>,
}

#[derive(Clone, Copy)]
struct RelationCase {
    id: &'static str,
    language: &'static str,
    text: &'static str,
    kind: TemporalRelationKindIR,
    left_fragment: &'static str,
    right_fragment: &'static str,
}

#[derive(Clone, Copy)]
struct TimeCase {
    id: &'static str,
    language: &'static str,
    text: &'static str,
    normalized: &'static str,
}

#[derive(Clone, Copy)]
struct QuestionCase {
    id: &'static str,
    language: LanguageCodeIR,
    statements: &'static [&'static str],
    question: &'static str,
    disposition: TemporalAnswerDispositionIR,
    event_evidence: usize,
    relation_evidence: usize,
    text_fragment: &'static str,
}

fn relation_cases() -> Vec<RelationCase> {
    use TemporalRelationKindIR::{Before, During, Simultaneous};
    vec![
        RelationCase {
            id: "REL_EN_INFIX_BEFORE",
            language: "ENGLISH_SURFACE",
            text: "The backup completed before the deploy started.",
            kind: Before,
            left_fragment: "backup",
            right_fragment: "deploy",
        },
        RelationCase {
            id: "REL_EN_INFIX_AFTER",
            language: "ENGLISH_SURFACE",
            text: "The deploy started after the backup completed.",
            kind: Before,
            left_fragment: "backup",
            right_fragment: "deploy",
        },
        RelationCase {
            id: "REL_EN_PREFIX_BEFORE",
            language: "ENGLISH_SURFACE",
            text: "Before the deploy started, the backup completed.",
            kind: Before,
            left_fragment: "backup",
            right_fragment: "deploy",
        },
        RelationCase {
            id: "REL_EN_PREFIX_AFTER",
            language: "ENGLISH_SURFACE",
            text: "After the backup completed, the deploy started.",
            kind: Before,
            left_fragment: "backup",
            right_fragment: "deploy",
        },
        RelationCase {
            id: "REL_EN_PREFIX_WHILE",
            language: "ENGLISH_SURFACE",
            text: "While the deploy was running, the monitor failed.",
            kind: Simultaneous,
            left_fragment: "deploy",
            right_fragment: "monitor",
        },
        RelationCase {
            id: "REL_EN_INFIX_WHILE",
            language: "ENGLISH_SURFACE",
            text: "The monitor failed while the deploy was running.",
            kind: Simultaneous,
            left_fragment: "monitor",
            right_fragment: "deploy",
        },
        RelationCase {
            id: "REL_EN_DURING",
            language: "ENGLISH_SURFACE",
            text: "The monitor failed during the deploy run.",
            kind: During,
            left_fragment: "monitor",
            right_fragment: "deploy",
        },
        RelationCase {
            id: "REL_KO_BEFORE",
            language: "KOREAN_SURFACE",
            text: "배포가 시작되기 전에 백업이 완료됐다.",
            kind: Before,
            left_fragment: "백업",
            right_fragment: "배포",
        },
        RelationCase {
            id: "REL_KO_AFTER",
            language: "KOREAN_SURFACE",
            text: "백업이 완료된 후에 배포가 시작됐다.",
            kind: Before,
            left_fragment: "백업",
            right_fragment: "배포",
        },
        RelationCase {
            id: "REL_KO_SINCE",
            language: "KOREAN_SURFACE",
            text: "백업이 완료된 이후에 배포가 시작됐다.",
            kind: Before,
            left_fragment: "백업",
            right_fragment: "배포",
        },
        RelationCase {
            id: "REL_KO_BEHIND",
            language: "KOREAN_SURFACE",
            text: "백업이 완료된 뒤에 배포가 시작됐다.",
            kind: Before,
            left_fragment: "백업",
            right_fragment: "배포",
        },
        RelationCase {
            id: "REL_KO_DURING",
            language: "KOREAN_SURFACE",
            text: "배포가 실행되는 동안 모니터가 실패했다.",
            kind: During,
            left_fragment: "모니터",
            right_fragment: "배포",
        },
        RelationCase {
            id: "REL_KO_SIMULTANEOUS",
            language: "KOREAN_SURFACE",
            text: "배포 시작과 동시에 모니터가 실행됐다.",
            kind: Simultaneous,
            left_fragment: "배포",
            right_fragment: "모니터",
        },
    ]
}

fn time_cases() -> Vec<TimeCase> {
    vec![
        TimeCase {
            id: "TIME_EN_YESTERDAY",
            language: "ENGLISH_SURFACE",
            text: "The backup completed yesterday.",
            normalized: "DAY_OFFSET:-1",
        },
        TimeCase {
            id: "TIME_EN_TODAY",
            language: "ENGLISH_SURFACE",
            text: "The backup completed today.",
            normalized: "DAY_OFFSET:0",
        },
        TimeCase {
            id: "TIME_EN_TOMORROW",
            language: "ENGLISH_SURFACE",
            text: "The deploy starts tomorrow.",
            normalized: "DAY_OFFSET:+1",
        },
        TimeCase {
            id: "TIME_EN_DAY_BEFORE",
            language: "ENGLISH_SURFACE",
            text: "The backup completed day before yesterday.",
            normalized: "DAY_OFFSET:-2",
        },
        TimeCase {
            id: "TIME_EN_DAY_AFTER",
            language: "ENGLISH_SURFACE",
            text: "The deploy starts day after tomorrow.",
            normalized: "DAY_OFFSET:+2",
        },
        TimeCase {
            id: "TIME_EN_LAST_WEEK",
            language: "ENGLISH_SURFACE",
            text: "The backup completed last week.",
            normalized: "WEEK_OFFSET:-1",
        },
        TimeCase {
            id: "TIME_EN_NEXT_WEEK",
            language: "ENGLISH_SURFACE",
            text: "The deploy starts next week.",
            normalized: "WEEK_OFFSET:+1",
        },
        TimeCase {
            id: "TIME_EN_ISO_DATE",
            language: "ENGLISH_SURFACE",
            text: "The backup completed on 2026-09-01.",
            normalized: "2026-09-01",
        },
        TimeCase {
            id: "TIME_EN_AM",
            language: "ENGLISH_SURFACE",
            text: "The backup completed at 3 am.",
            normalized: "TIME:03:00",
        },
        TimeCase {
            id: "TIME_EN_PM",
            language: "ENGLISH_SURFACE",
            text: "The deploy started at 3 pm.",
            normalized: "TIME:15:00",
        },
        TimeCase {
            id: "TIME_KO_YESTERDAY",
            language: "KOREAN_SURFACE",
            text: "백업이 어제 완료됐다.",
            normalized: "DAY_OFFSET:-1",
        },
        TimeCase {
            id: "TIME_KO_TODAY",
            language: "KOREAN_SURFACE",
            text: "백업이 오늘 완료됐다.",
            normalized: "DAY_OFFSET:0",
        },
        TimeCase {
            id: "TIME_KO_TOMORROW",
            language: "KOREAN_SURFACE",
            text: "배포가 내일 시작된다.",
            normalized: "DAY_OFFSET:+1",
        },
        TimeCase {
            id: "TIME_KO_LAST_WEEK",
            language: "KOREAN_SURFACE",
            text: "백업이 지난주에 완료됐다.",
            normalized: "WEEK_OFFSET:-1",
        },
        TimeCase {
            id: "TIME_KO_NEXT_WEEK",
            language: "KOREAN_SURFACE",
            text: "배포가 다음주에 시작된다.",
            normalized: "WEEK_OFFSET:+1",
        },
        TimeCase {
            id: "TIME_KO_DATE",
            language: "KOREAN_SURFACE",
            text: "백업이 2026년 9월 1일에 완료됐다.",
            normalized: "2026-09-01",
        },
        TimeCase {
            id: "TIME_KO_CLOCK",
            language: "KOREAN_SURFACE",
            text: "배포가 오후 3시에 시작됐다.",
            normalized: "TIME:15:00",
        },
    ]
}

fn question_cases() -> Vec<QuestionCase> {
    use LanguageCodeIR::{English, Korean};
    use TemporalAnswerDispositionIR::{
        AnsweredByTransitivePath, AnsweredFromTemporalGraph, NoMatchingEvent, NoRecordedRelation,
    };
    vec![
        QuestionCase {
            id: "QA_EN_WHEN_RELATIVE",
            language: English,
            statements: &["The backup completed yesterday."],
            question: "When did the backup complete?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 1,
            relation_evidence: 0,
            text_fragment: "DAY_OFFSET:-1",
        },
        QuestionCase {
            id: "QA_EN_WHEN_DATE",
            language: English,
            statements: &["The backup completed on 2026-09-01."],
            question: "When did the backup complete?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 1,
            relation_evidence: 0,
            text_fragment: "2026-09-01",
        },
        QuestionCase {
            id: "QA_EN_WHEN_CLOCK",
            language: English,
            statements: &["The deploy started at 3 pm."],
            question: "When did the deploy start?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 1,
            relation_evidence: 0,
            text_fragment: "TIME:15:00",
        },
        QuestionCase {
            id: "QA_EN_WHEN_TODAY",
            language: English,
            statements: &["The backup completed today."],
            question: "When did the backup complete?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 1,
            relation_evidence: 0,
            text_fragment: "DAY_OFFSET:0",
        },
        QuestionCase {
            id: "QA_KO_WHEN_RELATIVE",
            language: Korean,
            statements: &["백업이 어제 완료됐다."],
            question: "백업은 언제 완료됐어?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 1,
            relation_evidence: 0,
            text_fragment: "DAY_OFFSET:-1",
        },
        QuestionCase {
            id: "QA_KO_WHEN_DATE",
            language: Korean,
            statements: &["백업이 2026년 9월 1일에 완료됐다."],
            question: "백업은 언제 완료됐어?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 1,
            relation_evidence: 0,
            text_fragment: "2026-09-01",
        },
        QuestionCase {
            id: "QA_KO_WHEN_TODAY",
            language: Korean,
            statements: &["백업이 오늘 완료됐다."],
            question: "백업은 언제 완료됐어?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 1,
            relation_evidence: 0,
            text_fragment: "DAY_OFFSET:0",
        },
        QuestionCase {
            id: "QA_EN_BEFORE",
            language: English,
            statements: &["The backup completed before the deploy started."],
            question: "What happened before the deploy started?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 2,
            relation_evidence: 1,
            text_fragment: "temporal graph",
        },
        QuestionCase {
            id: "QA_EN_AFTER",
            language: English,
            statements: &["The backup completed before the deploy started."],
            question: "What happened after the backup completed?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 2,
            relation_evidence: 1,
            text_fragment: "dialogue evidence",
        },
        QuestionCase {
            id: "QA_EN_DURING",
            language: English,
            statements: &["The monitor failed during the deploy run."],
            question: "What happened during the deploy run?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 2,
            relation_evidence: 1,
            text_fragment: "evidence edge",
        },
        QuestionCase {
            id: "QA_KO_BEFORE",
            language: Korean,
            statements: &["배포가 시작되기 전에 백업이 완료됐다."],
            question: "배포가 시작되기 전에 무슨 일이 있었어?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 2,
            relation_evidence: 1,
            text_fragment: "시간 그래프",
        },
        QuestionCase {
            id: "QA_KO_AFTER",
            language: Korean,
            statements: &["백업이 완료된 후에 배포가 시작됐다."],
            question: "백업이 완료된 후에 무슨 일이 있었어?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 2,
            relation_evidence: 1,
            text_fragment: "실제 세계 사실 확정은 아니야",
        },
        QuestionCase {
            id: "QA_KO_DURING",
            language: Korean,
            statements: &["배포가 실행되는 동안 모니터가 실패했다."],
            question: "배포가 실행되는 동안 무슨 일이 있었어?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 2,
            relation_evidence: 1,
            text_fragment: "대화 근거",
        },
        QuestionCase {
            id: "QA_EN_RELATION",
            language: English,
            statements: &["The backup completed before the deploy started."],
            question: "Did the backup complete before the deploy start?",
            disposition: AnsweredFromTemporalGraph,
            event_evidence: 2,
            relation_evidence: 1,
            text_fragment: "not established world truth",
        },
        QuestionCase {
            id: "QA_EN_TRANSITIVE",
            language: English,
            statements: &[
                "The backup completed.",
                "After that, the deploy started.",
                "After that, the monitor failed.",
            ],
            question: "Did the backup complete before the monitor fail?",
            disposition: AnsweredByTransitivePath,
            event_evidence: 3,
            relation_evidence: 2,
            text_fragment: "2 evidence edge",
        },
        QuestionCase {
            id: "QA_UNKNOWN_EVENT",
            language: English,
            statements: &["The backup completed yesterday."],
            question: "What happened before the migration finished?",
            disposition: NoMatchingEvent,
            event_evidence: 0,
            relation_evidence: 0,
            text_fragment: "will not invent",
        },
        QuestionCase {
            id: "QA_UNRECORDED_ORDER",
            language: English,
            statements: &[
                "The backup completed yesterday.",
                "The deploy started today.",
            ],
            question: "Did the backup complete before the deploy start?",
            disposition: NoRecordedRelation,
            event_evidence: 2,
            relation_evidence: 0,
            text_fragment: "cannot infer the order",
        },
    ]
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

fn relation_rows() -> Vec<CanaryRow> {
    relation_cases()
        .into_iter()
        .enumerate()
        .map(|(index, case)| {
            let analysis = TemporalSemanticAnalyzer.analyze_turn(
                case.text,
                u64::try_from(index + 1).expect("bounded case"),
                None,
            );
            let relation = analysis.relations.first();
            let pass = relation.is_some_and(|relation| {
                relation.kind == case.kind
                    && analysis.events.len() == 2
                    && analysis.events[0].surface.contains(case.left_fragment)
                    && analysis.events[1].surface.contains(case.right_fragment)
                    && !relation.dialogue_truth_established
                    && !relation.external_execution_authorized
            });
            CanaryRow {
                id: case.id.to_string(),
                family: case.language.to_string(),
                input: case.text.to_string(),
                pass,
                observed: format!(
                    "events={:?};relation={:?}",
                    analysis
                        .events
                        .iter()
                        .map(|event| event.surface.as_str())
                        .collect::<Vec<_>>(),
                    relation.map(|relation| relation.kind)
                ),
            }
        })
        .collect()
}

fn time_rows() -> Vec<CanaryRow> {
    time_cases()
        .into_iter()
        .enumerate()
        .map(|(index, case)| {
            let analysis = TemporalSemanticAnalyzer.analyze_turn(
                case.text,
                u64::try_from(index + 1).expect("bounded case"),
                None,
            );
            let observed = analysis
                .events
                .first()
                .and_then(|event| event.event_time.as_ref())
                .map(|time| time.normalized_value.clone());
            CanaryRow {
                id: case.id.to_string(),
                family: "TEMPORAL_EXPRESSIONS".to_string(),
                input: case.text.to_string(),
                pass: observed.as_deref() == Some(case.normalized),
                observed: format!("language={};time={observed:?}", case.language),
            }
        })
        .collect()
}

fn question_rows() -> Vec<CanaryRow> {
    question_cases()
        .into_iter()
        .map(|case| {
            let mut api = CognitiveApi::new_embedded().expect("embedded API");
            let conversation_id = format!("R11-{}", case.id);
            for (index, statement) in case.statements.iter().enumerate() {
                api.process_conversation_turn(&request(
                    &conversation_id,
                    u64::try_from(index + 1).expect("bounded case"),
                    statement,
                    case.language,
                ))
                .expect("temporal seed");
            }
            let state_before = api
                .conversation_state(&conversation_id)
                .expect("seeded conversation")
                .clone();
            let response = api
                .process_conversation_turn(&request(
                    &conversation_id,
                    u64::try_from(case.statements.len() + 1).expect("bounded case"),
                    case.question,
                    case.language,
                ))
                .expect("temporal question");
            let answer = response.temporal_answer.as_ref();
            let pass = answer.is_some_and(|answer| {
                answer.validate()
                    && answer.disposition == case.disposition
                    && answer.event_evidence.len() == case.event_evidence
                    && answer.relation_evidence.len() == case.relation_evidence
                    && answer.realized_text.contains(case.text_fragment)
                    && !answer.dialogue_truth_established
                    && !answer.external_execution_authorized
            }) && response.conversation_state.temporal_graph
                == state_before.temporal_graph
                && response.conversation_state.active_goals.is_empty()
                && response.grounded_response.is_none()
                && response.output.grounded_plan_sha256.is_none()
                && response.output.unsupported_freeform_claims == 0;
            CanaryRow {
                id: case.id.to_string(),
                family: if case.statements.len() > 1 {
                    "CROSS_TURN_AND_TRANSITIVE"
                } else {
                    "TEMPORAL_QUESTIONS"
                }
                .to_string(),
                input: case.question.to_string(),
                pass,
                observed: answer.map_or_else(
                    || format!("answer=NONE;output={}", response.output.text),
                    |answer| {
                        format!(
                            "disposition={:?};events={};relations={};text={}",
                            answer.disposition,
                            answer.event_evidence.len(),
                            answer.relation_evidence.len(),
                            answer.realized_text
                        )
                    },
                ),
            }
        })
        .collect()
}

fn conflict_and_tamper_rows() -> Vec<CanaryRow> {
    let analyzer = TemporalSemanticAnalyzer;
    let mut graph = TemporalGraphIR::default();
    graph.apply_turn(&analyzer.analyze_turn(
        "The backup completed before the deploy started.",
        1,
        None,
    ));
    let snapshot = graph.clone();
    graph.apply_turn(&analyzer.analyze_turn(
        "The deploy started before the backup completed.",
        2,
        Some(&snapshot),
    ));
    let conflict_answer = TemporalQaEngine
        .answer(
            "Did the backup complete before the deploy start?",
            Some(&graph),
            LanguageCodeIR::English,
        )
        .expect("conflict answer");
    let mut rows = vec![CanaryRow {
        id: "SAFETY_CONFLICT_PRESERVED".to_string(),
        family: "CONFLICT_AND_TAMPER".to_string(),
        input: "A before B; B before A".to_string(),
        pass: conflict_answer.disposition == TemporalAnswerDispositionIR::ConflictingRelations
            && graph.conflicts.len() == 1
            && graph
                .relations
                .iter()
                .all(|relation| relation.status == TemporalRelationStatusIR::Contested)
            && graph.validate(2),
        observed: format!(
            "disposition={:?};conflicts={};statuses={:?}",
            conflict_answer.disposition,
            graph.conflicts.len(),
            graph
                .relations
                .iter()
                .map(|relation| relation.status)
                .collect::<Vec<_>>()
        ),
    }];

    let mut tampered = conflict_answer.clone();
    tampered.dialogue_truth_established = true;
    rows.push(tamper_row("TAMPER_ANSWER_TRUTH", !tampered.validate()));
    let mut tampered = conflict_answer.clone();
    tampered.external_execution_authorized = true;
    rows.push(tamper_row("TAMPER_ANSWER_AUTHORITY", !tampered.validate()));
    let mut tampered = conflict_answer.clone();
    tampered.event_evidence[0].dialogue_truth_established = true;
    rows.push(tamper_row("TAMPER_EVENT_TRUTH", !tampered.validate()));
    let mut tampered = conflict_answer.clone();
    tampered.relation_evidence[0].external_execution_authorized = true;
    rows.push(tamper_row(
        "TAMPER_RELATION_AUTHORITY",
        !tampered.validate(),
    ));
    let mut tampered = conflict_answer.clone();
    tampered.unsupported_claims = 1;
    rows.push(tamper_row("TAMPER_UNSUPPORTED_CLAIM", !tampered.validate()));
    let mut tampered_graph = graph.clone();
    tampered_graph.events[0].dialogue_truth_established = true;
    rows.push(tamper_row(
        "TAMPER_GRAPH_EVENT_TRUTH",
        !tampered_graph.validate(2),
    ));
    let mut tampered_graph = graph.clone();
    tampered_graph.relations[0].external_execution_authorized = true;
    rows.push(tamper_row(
        "TAMPER_GRAPH_RELATION_AUTHORITY",
        !tampered_graph.validate(2),
    ));
    let mut tampered_graph = graph.clone();
    tampered_graph.relations[0].right_event_id = "UNKNOWN-EVENT".to_string();
    rows.push(tamper_row(
        "TAMPER_UNKNOWN_ENDPOINT",
        !tampered_graph.validate(2),
    ));
    rows
}

fn tamper_row(id: &str, pass: bool) -> CanaryRow {
    CanaryRow {
        id: id.to_string(),
        family: "CONFLICT_AND_TAMPER".to_string(),
        input: id.to_string(),
        pass,
        observed: format!("tamper_rejected={pass}"),
    }
}

fn main() {
    let mut rows = relation_rows();
    rows.extend(time_rows());
    rows.extend(question_rows());
    rows.extend(conflict_and_tamper_rows());
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = CanaryReport {
        schema: "B_CORE_TEMPORAL_DISCOURSE_CANARY_V1",
        status: if passed == rows.len() { "PASS" } else { "FAIL" },
        total: rows.len(),
        passed,
        failed: rows.len() - passed,
        english_surface: rows
            .iter()
            .filter(|row| row.family == "ENGLISH_SURFACE")
            .count(),
        korean_surface: rows
            .iter()
            .filter(|row| row.family == "KOREAN_SURFACE")
            .count(),
        temporal_expressions: rows
            .iter()
            .filter(|row| row.family == "TEMPORAL_EXPRESSIONS")
            .count(),
        temporal_questions: rows
            .iter()
            .filter(|row| row.family == "TEMPORAL_QUESTIONS")
            .count(),
        cross_turn_and_transitive: rows
            .iter()
            .filter(|row| row.family == "CROSS_TURN_AND_TRANSITIVE")
            .count(),
        conflict_and_tamper: rows
            .iter()
            .filter(|row| row.family == "CONFLICT_AND_TAMPER")
            .count(),
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string(&report).expect("serialize temporal canary")
    );
    if report.failed != 0 {
        std::process::exit(1);
    }
}
