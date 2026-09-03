//! Frozen R64 readiness suite: five structural variants for each diagnostic family.
//! Product code must not contain any of these whole utterances.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, ConversationTurnResponseIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone)]
struct Turn {
    text: String,
    language: LanguageCodeIR,
}

impl Turn {
    fn en(text: &str) -> Self {
        Self {
            text: text.to_string(),
            language: LanguageCodeIR::English,
        }
    }

    fn ko(text: &str) -> Self {
        Self {
            text: text.to_string(),
            language: LanguageCodeIR::Korean,
        }
    }
}

#[derive(Clone)]
enum Expectation {
    Goal {
        turn: usize,
        intent: &'static str,
        target: String,
        rejected: String,
        min_blocked: usize,
    },
    ReportThenUnavailableResult {
        report_turn: usize,
        result_turn: usize,
        target: String,
    },
    MultiGoal {
        turn: usize,
        predicates: Vec<&'static str>,
        targets: Vec<String>,
        forbidden_output: Vec<&'static str>,
    },
}

#[derive(Clone)]
struct Family {
    id: &'static str,
    axis: &'static str,
    base: Vec<Turn>,
    paraphrase: Vec<Turn>,
    reordered: Vec<Turn>,
    mirror: Vec<Turn>,
    replacements: Vec<(&'static str, &'static str)>,
    expectation: Expectation,
}

#[derive(Clone)]
struct Case {
    id: String,
    axis: &'static str,
    transform: &'static str,
    turns: Vec<Turn>,
    expectation: Expectation,
}

#[derive(Serialize)]
struct Row {
    id: String,
    axis: String,
    transform: String,
    pass: bool,
    trace: String,
}

#[derive(Serialize)]
struct Report {
    schema: &'static str,
    suite: &'static str,
    frozen_before_further_product_repairs: bool,
    family_count: usize,
    transforms_per_family: usize,
    passed: usize,
    failed: usize,
    total: usize,
    rows: Vec<Row>,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
}

fn goal(
    turn: usize,
    intent: &'static str,
    target: &str,
    rejected: &str,
    min_blocked: usize,
) -> Expectation {
    Expectation::Goal {
        turn,
        intent,
        target: target.to_string(),
        rejected: rejected.to_string(),
        min_blocked,
    }
}

fn report_result(target: &str) -> Expectation {
    Expectation::ReportThenUnavailableResult {
        report_turn: 2,
        result_turn: 3,
        target: target.to_string(),
    }
}

fn multi(targets: &[&str]) -> Expectation {
    Expectation::MultiGoal {
        turn: 1,
        predicates: vec!["INVESTIGATE", "REPAIR"],
        targets: targets.iter().map(|target| (*target).to_string()).collect(),
        forbidden_output: vec!["GoalIR", "completed", "완료했"],
    }
}

fn families() -> Vec<Family> {
    vec![
        Family {
            id: "D01",
            axis: "GRAMMAR_SCOPE",
            base: vec![Turn::en("Could you take a look at the Aster cache?")],
            paraphrase: vec![Turn::en("Would you inspect the Aster cache, please?")],
            reordered: vec![Turn::en("The Aster cache—could you take a look?")],
            mirror: vec![Turn::ko("Aster 캐시 좀 봐줄래?")],
            replacements: vec![("Aster", "Beryl")],
            expectation: goal(1, "INVESTIGATE", "Aster", "", 0),
        },
        Family {
            id: "D02",
            axis: "GRAMMAR_SCOPE",
            base: vec![Turn::ko("Aster 캐시 좀 봐줄래?")],
            paraphrase: vec![Turn::ko("Aster 캐시를 살펴봐 줄 수 있어?")],
            reordered: vec![Turn::ko("Aster 캐시 말인데, 좀 확인해 줄래?")],
            mirror: vec![Turn::en("Could you have a look at the Aster cache?")],
            replacements: vec![("Aster", "Beryl")],
            expectation: goal(1, "INVESTIGATE", "Aster", "", 0),
        },
        Family {
            id: "D03",
            axis: "GRAMMAR_SCOPE",
            base: vec![Turn::en(
                "Inspect the Birch log now, but repair the Cedar queue only if the cache is stale",
            )],
            paraphrase: vec![Turn::en(
                "Check Birch now; fix Cedar only when the cache is stale",
            )],
            reordered: vec![Turn::en(
                "Repair the Cedar queue only if the cache is stale, but inspect the Birch log now",
            )],
            mirror: vec![Turn::ko(
                "Birch 로그는 지금 조사하되 캐시가 오래됐을 때만 Cedar 큐를 수리해",
            )],
            replacements: vec![("Birch", "Cobalt"), ("Cedar", "Dahlia")],
            expectation: goal(1, "INVESTIGATE", "Birch", "Cedar", 0),
        },
        Family {
            id: "D04",
            axis: "GRAMMAR_SCOPE",
            base: vec![Turn::ko(
                "Birch 로그는 지금 조사하되 캐시가 오래됐을 때만 Cedar 큐를 수리해",
            )],
            paraphrase: vec![Turn::ko(
                "Birch 로그는 바로 확인하고 캐시가 낡은 경우에만 Cedar 큐를 고쳐",
            )],
            reordered: vec![Turn::ko(
                "캐시가 오래됐을 때만 Cedar 큐를 수리하되 Birch 로그는 지금 조사해",
            )],
            mirror: vec![Turn::en(
                "Inspect the Birch log now, but repair the Cedar queue only if the cache is stale",
            )],
            replacements: vec![("Birch", "Cobalt"), ("Cedar", "Dahlia")],
            expectation: goal(1, "INVESTIGATE", "Birch", "Cedar", 0),
        },
        Family {
            id: "D05",
            axis: "GRAMMAR_SCOPE",
            base: vec![Turn::en(
                "Unless the Dune service is healthy, repair the Ember worker; inspect the Flint report now",
            )],
            paraphrase: vec![Turn::en(
                "If Dune is not healthy, fix Ember; review the Flint report now",
            )],
            reordered: vec![Turn::en(
                "Inspect the Flint report now; unless Dune is healthy, repair the Ember worker",
            )],
            mirror: vec![Turn::ko(
                "Dune 서비스가 정상이 아닌 경우에만 Ember 워커를 수리하고 지금은 Flint 보고서를 조사해",
            )],
            replacements: vec![
                ("Dune", "Elm"),
                ("Ember", "Fennel"),
                ("Flint", "Grove"),
            ],
            expectation: goal(1, "INVESTIGATE", "Flint", "Ember", 0),
        },
        Family {
            id: "D06",
            axis: "GRAMMAR_SCOPE",
            base: vec![Turn::ko(
                "Dune 서비스가 정상이 아닌 경우에만 Ember 워커를 수리하고 지금은 Flint 보고서를 조사해",
            )],
            paraphrase: vec![Turn::ko(
                "Dune 서비스가 건강하지 않을 때만 Ember 워커를 고치고 Flint 보고서는 지금 검토해",
            )],
            reordered: vec![Turn::ko(
                "Flint 보고서는 지금 조사하고 Dune 서비스가 정상이 아니면 Ember 워커를 수리해",
            )],
            mirror: vec![Turn::en(
                "Unless the Dune service is healthy, repair the Ember worker; inspect the Flint report now",
            )],
            replacements: vec![
                ("Dune", "Elm"),
                ("Ember", "Fennel"),
                ("Flint", "Grove"),
            ],
            expectation: goal(1, "INVESTIGATE", "Flint", "Ember", 0),
        },
        Family {
            id: "D07",
            axis: "PRAGMATIC_INTENT",
            base: vec![Turn::en(
                "Even if the Garnet cache failed, do not delete it; explain why it failed",
            )],
            paraphrase: vec![Turn::en(
                "Although Garnet failed, never remove it; describe why Garnet failed",
            )],
            reordered: vec![Turn::en(
                "Explain why Garnet failed; even if its cache failed, do not delete it",
            )],
            mirror: vec![Turn::ko(
                "Garnet 캐시가 실패했더라도 그걸 삭제하지 말고 왜 실패했는지 설명해",
            )],
            replacements: vec![("Garnet", "Hazel")],
            expectation: goal(1, "EXPLAIN", "Garnet", "", 1),
        },
        Family {
            id: "D08",
            axis: "PRAGMATIC_INTENT",
            base: vec![Turn::ko(
                "Garnet 캐시가 실패했더라도 그걸 삭제하지 말고 왜 실패했는지 설명해",
            )],
            paraphrase: vec![Turn::ko(
                "Garnet 캐시가 망가졌어도 지우지는 말고 실패 원인을 설명해",
            )],
            reordered: vec![Turn::ko(
                "Garnet 실패 원인을 설명하되 캐시는 실패했어도 삭제하지 마",
            )],
            mirror: vec![Turn::en(
                "Even if the Garnet cache failed, do not delete it; explain why it failed",
            )],
            replacements: vec![("Garnet", "Hazel")],
            expectation: goal(1, "EXPLAIN", "Garnet", "", 1),
        },
        Family {
            id: "D09",
            axis: "REFERENCE_ELLIPSIS",
            base: vec![Turn::en(
                "Not the Ivory index—the Juniper queue. Repair that one",
            )],
            paraphrase: vec![Turn::en(
                "Repair the Juniper queue, not the Ivory index",
            )],
            reordered: vec![Turn::en(
                "Repair that one: the Juniper queue, not the Ivory index",
            )],
            mirror: vec![Turn::ko(
                "Ivory 인덱스 말고 Juniper 큐야. 그걸 수리해",
            )],
            replacements: vec![("Ivory", "Indigo"), ("Juniper", "Kite")],
            expectation: goal(1, "REPAIR", "Juniper", "Ivory", 0),
        },
        Family {
            id: "D10",
            axis: "REFERENCE_ELLIPSIS",
            base: vec![Turn::ko(
                "Ivory 인덱스 말고 Juniper 큐야. 그걸 수리해",
            )],
            paraphrase: vec![Turn::ko(
                "Ivory 인덱스가 아니라 Juniper 큐를 수리해",
            )],
            reordered: vec![Turn::ko(
                "그걸 수리해. Ivory 인덱스 말고 Juniper 큐를 말하는 거야",
            )],
            mirror: vec![Turn::en(
                "Not the Ivory index—the Juniper queue. Repair that one",
            )],
            replacements: vec![("Ivory", "Indigo"), ("Juniper", "Kite")],
            expectation: goal(1, "REPAIR", "Juniper", "Ivory", 0),
        },
        Family {
            id: "D11",
            axis: "REFERENCE_ELLIPSIS",
            base: vec![
                Turn::en("Inspect the Kestrel worker"),
                Turn::en("Do the same to the Linen queue"),
            ],
            paraphrase: vec![
                Turn::en("Review the Kestrel worker"),
                Turn::en("Apply that operation to the Linen queue as well"),
            ],
            reordered: vec![
                Turn::en("The Kestrel worker—inspect it"),
                Turn::en("For the Linen queue, do the same"),
            ],
            mirror: vec![
                Turn::ko("Kestrel 워커를 조사해"),
                Turn::ko("Linen 큐에도 똑같이 해"),
            ],
            replacements: vec![("Kestrel", "Larch"), ("Linen", "Maple")],
            expectation: goal(2, "INVESTIGATE", "Linen", "Kestrel", 0),
        },
        Family {
            id: "D12",
            axis: "REFERENCE_ELLIPSIS",
            base: vec![
                Turn::ko("Kestrel 워커를 조사해"),
                Turn::ko("Linen 큐에도 똑같이 해"),
            ],
            paraphrase: vec![
                Turn::ko("Kestrel 워커를 확인해"),
                Turn::ko("Linen 큐에도 같은 작업을 적용해"),
            ],
            reordered: vec![
                Turn::ko("Kestrel 워커 말인데, 그걸 조사해"),
                Turn::ko("같은 조사를 Linen 큐에도 해"),
            ],
            mirror: vec![
                Turn::en("Inspect the Kestrel worker"),
                Turn::en("Do the same to the Linen queue"),
            ],
            replacements: vec![("Kestrel", "Larch"), ("Linen", "Maple")],
            expectation: goal(2, "INVESTIGATE", "Linen", "Kestrel", 0),
        },
        Family {
            id: "D13",
            axis: "DISCOURSE_TOPIC",
            base: vec![Turn::en(
                "The Mallow service keeps timing out. Find out why",
            )],
            paraphrase: vec![Turn::en(
                "The Mallow service repeatedly times out. Investigate the cause",
            )],
            reordered: vec![Turn::en(
                "Find out why: the Mallow service keeps timing out",
            )],
            mirror: vec![Turn::ko(
                "Mallow 서비스가 계속 시간 초과돼. 원인을 확인해 줘",
            )],
            replacements: vec![("Mallow", "Nettle")],
            expectation: goal(1, "INVESTIGATE", "Mallow", "", 0),
        },
        Family {
            id: "D14",
            axis: "DISCOURSE_TOPIC",
            base: vec![Turn::ko(
                "Navy 서비스가 계속 시간 초과돼. 원인을 확인해 줘",
            )],
            paraphrase: vec![Turn::ko(
                "Navy 서비스에서 시간 초과가 반복돼. 왜 그런지 조사해",
            )],
            reordered: vec![Turn::ko(
                "원인을 확인해 줘. Navy 서비스가 계속 시간 초과돼",
            )],
            mirror: vec![Turn::en(
                "The Navy service keeps timing out. Find out why",
            )],
            replacements: vec![("Navy", "Opal")],
            expectation: goal(1, "INVESTIGATE", "Navy", "", 0),
        },
        Family {
            id: "D15",
            axis: "PLAN_RESULT_BOUNDARY",
            base: vec![
                Turn::en("Run the Ocher migration"),
                Turn::en("Someone said it finished"),
                Turn::en(
                    "I do not need the claim. Tell me whether the actual result was verified",
                ),
            ],
            paraphrase: vec![
                Turn::en("Execute the Ocher migration"),
                Turn::en("A teammate reported that it completed"),
                Turn::en("Ignore that report. Was the real Ocher result actually verified?"),
            ],
            reordered: vec![
                Turn::en("Run the Ocher migration"),
                Turn::en("It was said to be finished"),
                Turn::en("Was the actual result verified? Ignore the claim"),
            ],
            mirror: vec![
                Turn::ko("Ocher 마이그레이션을 실행해"),
                Turn::ko("누가 그게 끝났다고 했어"),
                Turn::ko("그 주장은 필요 없어. 실제 결과가 검증됐는지 알려줘"),
            ],
            replacements: vec![("Ocher", "Pine")],
            expectation: report_result("Ocher"),
        },
        Family {
            id: "D16",
            axis: "PLAN_RESULT_BOUNDARY",
            base: vec![
                Turn::ko("Ocher 마이그레이션을 실행해"),
                Turn::ko("누가 그게 끝났다고 했어"),
                Turn::ko("그 주장은 필요 없어. 실제 결과가 검증됐는지 알려줘"),
            ],
            paraphrase: vec![
                Turn::ko("Ocher 마이그레이션을 수행해"),
                Turn::ko("동료가 완료됐다고 보고했어"),
                Turn::ko("보고는 빼고 실제 Ocher 결과가 검증됐는지 말해줘"),
            ],
            reordered: vec![
                Turn::ko("Ocher 마이그레이션을 실행해"),
                Turn::ko("끝났다는 말을 들었어"),
                Turn::ko("실제 결과가 검증됐어? 그 주장은 제외해"),
            ],
            mirror: vec![
                Turn::en("Run the Ocher migration"),
                Turn::en("Someone said it finished"),
                Turn::en(
                    "I do not need the claim. Tell me whether the actual result was verified",
                ),
            ],
            replacements: vec![("Ocher", "Pine")],
            expectation: report_result("Ocher"),
        },
        Family {
            id: "D17",
            axis: "DISCOURSE_TOPIC",
            base: vec![
                Turn::en("Inspect the Parchment cache"),
                Turn::en("Inspect the Quartz queue"),
                Turn::en("Go back to the first issue and explain why it failed"),
            ],
            paraphrase: vec![
                Turn::en("Review the Parchment cache"),
                Turn::en("Review the Quartz queue"),
                Turn::en("Return to issue one and explain its failure"),
            ],
            reordered: vec![
                Turn::en("Inspect the Parchment cache"),
                Turn::en("Inspect the Quartz queue"),
                Turn::en("Explain why the first issue failed; go back to it"),
            ],
            mirror: vec![
                Turn::ko("Parchment 캐시를 조사해"),
                Turn::ko("Quartz 큐를 조사해"),
                Turn::ko("첫 번째 문제로 돌아가서 왜 실패했는지 설명해"),
            ],
            replacements: vec![("Parchment", "Raven"), ("Quartz", "Spruce")],
            expectation: goal(3, "EXPLAIN", "Parchment", "Quartz", 0),
        },
        Family {
            id: "D18",
            axis: "DISCOURSE_TOPIC",
            base: vec![
                Turn::ko("Parchment 캐시를 조사해"),
                Turn::ko("Quartz 큐를 조사해"),
                Turn::ko("첫 번째 문제로 돌아가서 왜 실패했는지 설명해"),
            ],
            paraphrase: vec![
                Turn::ko("Parchment 캐시를 확인해"),
                Turn::ko("Quartz 큐를 확인해"),
                Turn::ko("1번 문제로 되돌아가 실패 원인을 설명해"),
            ],
            reordered: vec![
                Turn::ko("Parchment 캐시를 조사해"),
                Turn::ko("Quartz 큐를 조사해"),
                Turn::ko("왜 실패했는지 설명해. 첫 번째 문제로 돌아가"),
            ],
            mirror: vec![
                Turn::en("Inspect the Parchment cache"),
                Turn::en("Inspect the Quartz queue"),
                Turn::en("Go back to the first issue and explain why it failed"),
            ],
            replacements: vec![("Parchment", "Raven"), ("Quartz", "Spruce")],
            expectation: goal(3, "EXPLAIN", "Parchment", "Quartz", 0),
        },
        Family {
            id: "D19",
            axis: "GROUNDED_REALIZATION",
            base: vec![Turn::en(
                "Inspect the Rose cache and the Sienna queue, but repair only the latter",
            )],
            paraphrase: vec![Turn::en(
                "Review the Rose cache together with the Sienna queue, then fix just the second item",
            )],
            reordered: vec![Turn::en(
                "Repair only the latter, but first inspect the Rose cache and the Sienna queue",
            )],
            mirror: vec![Turn::ko(
                "Rose 캐시와 Sienna 큐를 조사하되 후자만 수리해",
            )],
            replacements: vec![("Rose", "Tulip"), ("Sienna", "Umber")],
            expectation: multi(&["Rose", "Sienna"]),
        },
        Family {
            id: "D20",
            axis: "GROUNDED_REALIZATION",
            base: vec![Turn::ko(
                "Rose 캐시와 Sienna 큐를 조사하되 후자만 수리해",
            )],
            paraphrase: vec![Turn::ko(
                "Rose 캐시와 Sienna 큐를 함께 확인하고 두 번째 것만 고쳐",
            )],
            reordered: vec![Turn::ko(
                "후자만 수리하되 먼저 Rose 캐시와 Sienna 큐를 조사해",
            )],
            mirror: vec![Turn::en(
                "Inspect the Rose cache and the Sienna queue, but repair only the latter",
            )],
            replacements: vec![("Rose", "Tulip"), ("Sienna", "Umber")],
            expectation: multi(&["Rose", "Sienna"]),
        },
    ]
}

fn replace_turns(turns: &[Turn], replacements: &[(&str, &str)]) -> Vec<Turn> {
    turns
        .iter()
        .map(|turn| {
            let text = replacements
                .iter()
                .fold(turn.text.clone(), |text, (from, to)| text.replace(from, to));
            Turn {
                text,
                language: turn.language,
            }
        })
        .collect()
}

fn replace_expectation(expectation: &Expectation, replacements: &[(&str, &str)]) -> Expectation {
    let replace = |text: &str| {
        replacements
            .iter()
            .fold(text.to_string(), |text, (from, to)| text.replace(from, to))
    };
    match expectation {
        Expectation::Goal {
            turn,
            intent,
            target,
            rejected,
            min_blocked,
        } => Expectation::Goal {
            turn: *turn,
            intent,
            target: replace(target),
            rejected: replace(rejected),
            min_blocked: *min_blocked,
        },
        Expectation::ReportThenUnavailableResult {
            report_turn,
            result_turn,
            target,
        } => Expectation::ReportThenUnavailableResult {
            report_turn: *report_turn,
            result_turn: *result_turn,
            target: replace(target),
        },
        Expectation::MultiGoal {
            turn,
            predicates,
            targets,
            forbidden_output,
        } => Expectation::MultiGoal {
            turn: *turn,
            predicates: predicates.clone(),
            targets: targets.iter().map(|target| replace(target)).collect(),
            forbidden_output: forbidden_output.clone(),
        },
    }
}

fn distract(turns: &[Turn]) -> Vec<Turn> {
    let mut result = turns.to_vec();
    if let Some(first) = result.first_mut() {
        let prefix = match first.language {
            LanguageCodeIR::Korean => "참고 맥락: 현재 사안에 관한 말이야. ",
            LanguageCodeIR::English => "Context note: this concerns the current matter. ",
            LanguageCodeIR::Mixed | LanguageCodeIR::Unknown => {
                "Context note: this concerns the current matter. "
            }
        };
        first.text = format!("{prefix}{}", first.text);
    }
    result
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::with_capacity(100);
    for family in families() {
        cases.push(Case {
            id: format!("R64_M{}_ENTITY_RENAME", family.id),
            axis: family.axis,
            transform: "ENTITY_RENAME",
            turns: replace_turns(&family.base, &family.replacements),
            expectation: replace_expectation(&family.expectation, &family.replacements),
        });
        cases.push(Case {
            id: format!("R64_M{}_LEXICAL_PARAPHRASE", family.id),
            axis: family.axis,
            transform: "LEXICAL_PARAPHRASE",
            turns: family.paraphrase.clone(),
            expectation: family.expectation.clone(),
        });
        cases.push(Case {
            id: format!("R64_M{}_CLAUSE_ORDER", family.id),
            axis: family.axis,
            transform: "CLAUSE_ORDER",
            turns: family.reordered.clone(),
            expectation: family.expectation.clone(),
        });
        cases.push(Case {
            id: format!("R64_M{}_DISTRACTOR", family.id),
            axis: family.axis,
            transform: "DISTRACTOR",
            turns: distract(&family.base),
            expectation: family.expectation.clone(),
        });
        cases.push(Case {
            id: format!("R64_M{}_LANGUAGE_MIRROR", family.id),
            axis: family.axis,
            transform: "LANGUAGE_MIRROR",
            turns: family.mirror.clone(),
            expectation: family.expectation,
        });
    }
    assert_eq!(cases.len(), 100, "fixed metamorphic denominator");
    cases
}

fn request(case_id: &str, turn_index: usize, turn: &Turn) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: case_id.to_string(),
        turn_index: u64::try_from(turn_index).expect("bounded turn index"),
        request_id: format!("{case_id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: turn.text.clone(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(turn.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn contains_ci(text: &str, expected: &str) -> bool {
    text.to_lowercase().contains(&expected.to_lowercase())
}

fn check(expectation: &Expectation, responses: &[ConversationTurnResponseIR]) -> (bool, String) {
    match expectation {
        Expectation::Goal {
            turn,
            intent,
            target,
            rejected,
            min_blocked,
        } => {
            let response = &responses[turn - 1];
            let grounded = response.grounded_response.as_deref();
            let actual_intent = grounded
                .map(|item| format!("{:?}", item.understanding.intent).to_uppercase())
                .unwrap_or_default();
            let subject = grounded
                .map(|item| item.understanding.subject.as_str())
                .unwrap_or_default();
            let blocked = response
                .pragmatic_interpretation
                .compositional_analysis
                .blocked_execution_count();
            let pass = response.disposition == ConversationTurnDispositionIR::Grounded
                && grounded.is_some()
                && actual_intent == *intent
                && contains_ci(subject, target)
                && (rejected.is_empty() || !contains_ci(subject, rejected))
                && blocked >= *min_blocked;
            (
                pass,
                format!(
                    "intent={actual_intent};subject={subject};blocked={blocked};resolved={}",
                    response.reference_resolution.resolved_semantic_text
                ),
            )
        }
        Expectation::ReportThenUnavailableResult {
            report_turn,
            result_turn,
            target,
        } => {
            let report = &responses[report_turn - 1];
            let result = &responses[result_turn - 1];
            let reports = report
                .conversation_state
                .action_state_ledger
                .language_report_history
                .len();
            let unavailable = !result.plan_result_boundary.snapshots.is_empty()
                && result
                    .plan_result_boundary
                    .snapshots
                    .iter()
                    .all(|snapshot| format!("{:?}", snapshot.result_availability) == "Unavailable");
            let pass = reports > 0
                && unavailable
                && contains_ci(&result.output.text, target)
                && result.output.unsupported_freeform_claims == 0;
            (
                pass,
                format!(
                    "reports={reports};unavailable={unavailable};text={}",
                    result.output.text
                ),
            )
        }
        Expectation::MultiGoal {
            turn,
            predicates,
            targets,
            forbidden_output,
        } => {
            let response = &responses[turn - 1];
            let composition = response
                .pragmatic_interpretation
                .pragmatic_intent_graph
                .composition
                .as_ref();
            let selected = composition
                .map(|graph| {
                    graph
                        .nodes
                        .iter()
                        .filter(|node| graph.selected_node_ids.contains(&node.node_id))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let selected_predicates = selected
                .iter()
                .map(|node| node.canonical_predicate.as_str())
                .collect::<Vec<_>>();
            let selected_subjects = selected
                .iter()
                .map(|node| node.subject.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
            let predicates_ok = predicates.iter().all(|predicate| {
                selected_predicates
                    .iter()
                    .any(|actual| actual.eq_ignore_ascii_case(predicate))
            });
            let targets_ok = targets.iter().all(|target| {
                contains_ci(&selected_subjects, target)
                    && contains_ci(&response.output.text, target)
            });
            let output_ok = forbidden_output
                .iter()
                .all(|forbidden| !contains_ci(&response.output.text, forbidden));
            (
                predicates_ok && targets_ok && output_ok,
                format!(
                    "predicates={selected_predicates:?};subjects={selected_subjects};text={}",
                    response.output.text
                ),
            )
        }
    }
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut responses = Vec::new();
    let mut contracts = Vec::new();
    for (offset, turn) in case.turns.iter().enumerate() {
        let request = request(&case.id, offset + 1, turn);
        let response = api
            .process_conversation_turn(&request)
            .expect("conversation turn");
        contracts.push(
            response.validate_against(&request)
                && response.output.unsupported_freeform_claims == 0
                && response.six_axis_integration.complete
                && !response.six_axis_integration.semantic_authority
                && !response.six_axis_integration.language_can_execute,
        );
        responses.push(response);
    }
    let (expected, detail) = check(&case.expectation, &responses);
    let pass = expected && contracts.iter().all(|contract| *contract);
    Row {
        id: case.id,
        axis: case.axis.to_string(),
        transform: case.transform.to_string(),
        pass,
        trace: format!("contracts={contracts:?};expected={expected};{detail}"),
    }
}

fn main() {
    let rows = cases().into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = Report {
        schema: "B_CORE_LANGUAGE_COMPLETION_METAMORPHIC_REPORT_1",
        suite: "R64-LANGUAGE-COMPLETION-METAMORPHIC-READINESS",
        frozen_before_further_product_repairs: true,
        family_count: 20,
        transforms_per_family: 5,
        passed,
        failed: rows.len() - passed,
        total: rows.len(),
        rows,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
    };
    println!("{}", serde_json::to_string_pretty(&report).expect("report"));
    if report.failed > 0 {
        std::process::exit(1);
    }
}
