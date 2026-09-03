//! Fresh productive probes for the response-boundary failures that dominated
//! the frozen GPT-reference V3 result.  The cases use new utterances and
//! entities.  They measure structural transfer and are not a replacement GPT
//! final score.

use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    NativeResponseGoalIR, NaturalResponseActIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
struct Turn {
    text: &'static str,
    language: LanguageCodeIR,
}

#[derive(Clone, Copy)]
enum Expected {
    Plan(PlanIntentIR, &'static str),
    ResultAbsence(&'static str),
    Act(NativeResponseGoalIR, NaturalResponseActIR),
}

struct Case {
    id: &'static str,
    family: &'static str,
    setup: Vec<Turn>,
    query: Turn,
    expected: Expected,
}

#[derive(Serialize)]
struct Row {
    id: String,
    family: String,
    pass: bool,
    response_goal: String,
    response_act: String,
    selected_goals: Vec<String>,
    ambiguity_count: usize,
    unresolved_count: usize,
    reference_ambiguities: Vec<String>,
    native_unresolved: Vec<String>,
    pragmatic_unresolved: Vec<String>,
    output: String,
    safety_pass: bool,
}

#[derive(Serialize)]
struct Report {
    schema: &'static str,
    suite: &'static str,
    frozen_v3_prompt_reuse: bool,
    final_gpt_score_claimed: bool,
    cases: usize,
    passed: usize,
    failed: usize,
    pass_rate_basis_points: usize,
    family_scores: Vec<FamilyScore>,
    safety_violations: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

#[derive(Serialize)]
struct FamilyScore {
    family: String,
    passed: usize,
    total: usize,
}

fn en(text: &'static str) -> Turn {
    Turn {
        text,
        language: LanguageCodeIR::English,
    }
}

fn ko(text: &'static str) -> Turn {
    Turn {
        text,
        language: LanguageCodeIR::Korean,
    }
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "P01",
            family: "PRODUCTIVE_REQUEST",
            setup: vec![],
            query: en("Could you take a diagnostic pass over the Basalt gateway?"),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Basalt"),
        },
        Case {
            id: "P02",
            family: "PRODUCTIVE_REQUEST",
            setup: vec![],
            query: en("I'd like you to map out a repair for the Cobalt index."),
            expected: Expected::Plan(PlanIntentIR::Repair, "Cobalt"),
        },
        Case {
            id: "P03",
            family: "PRODUCTIVE_REQUEST",
            setup: vec![],
            query: en("Can you give me a plain explanation of the Flint pipeline check?"),
            expected: Expected::Plan(PlanIntentIR::Explain, "Flint"),
        },
        Case {
            id: "P04",
            family: "PRODUCTIVE_REQUEST",
            setup: vec![],
            query: en("Please make the Garnet scheduler diagnosis our starting point."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Garnet"),
        },
        Case {
            id: "P05",
            family: "PRODUCTIVE_REQUEST",
            setup: vec![],
            query: ko("Helix 큐 원인부터 좀 좁혀 줄래?"),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Helix"),
        },
        Case {
            id: "P06",
            family: "PRODUCTIVE_REQUEST",
            setup: vec![],
            query: ko("Indigo 워커를 되살릴 순서를 짜 줘."),
            expected: Expected::Plan(PlanIntentIR::Repair, "Indigo"),
        },
        Case {
            id: "C01",
            family: "CONSTRAINT_INHERITANCE",
            setup: vec![en("Inspect the Juniper gateway.")],
            query: en("Keep it read-only and do not change it."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Juniper"),
        },
        Case {
            id: "C02",
            family: "CONSTRAINT_INHERITANCE",
            setup: vec![en("Diagnose the Kestrel queue.")],
            query: en("Only observe it; leave everything untouched."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Kestrel"),
        },
        Case {
            id: "C03",
            family: "CONSTRAINT_INHERITANCE",
            setup: vec![en("Repair the Lumen relay.")],
            query: en("For now, just prepare the steps without applying them."),
            expected: Expected::Plan(PlanIntentIR::Repair, "Lumen"),
        },
        Case {
            id: "C04",
            family: "CONSTRAINT_INHERITANCE",
            setup: vec![ko("Mica 로그를 조사해.")],
            query: ko("읽기만 하고 아무것도 바꾸지는 마."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Mica"),
        },
        Case {
            id: "C05",
            family: "CONSTRAINT_INHERITANCE",
            setup: vec![ko("Nickel 서비스를 진단해.")],
            query: ko("관찰만 해. 수정은 아직 하지 말고."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Nickel"),
        },
        Case {
            id: "C06",
            family: "CONSTRAINT_INHERITANCE",
            setup: vec![ko("Onyx 파일을 복구해.")],
            query: ko("실행하지 말고 복구 순서만 준비해."),
            expected: Expected::Plan(PlanIntentIR::Repair, "Onyx"),
        },
        Case {
            id: "E01",
            family: "OPERATION_ELLIPSIS",
            setup: vec![en("Inspect the Quartz cache and the Radian worker.")],
            query: en("Start with the latter one."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Radian"),
        },
        Case {
            id: "E02",
            family: "OPERATION_ELLIPSIS",
            setup: vec![en("Repair the Slate queue.")],
            query: en("Do that one first."),
            expected: Expected::Plan(PlanIntentIR::Repair, "Slate"),
        },
        Case {
            id: "E03",
            family: "OPERATION_ELLIPSIS",
            setup: vec![en("Explain the Topaz scheduler.")],
            query: en("Go ahead with that."),
            expected: Expected::Plan(PlanIntentIR::Explain, "Topaz"),
        },
        Case {
            id: "E04",
            family: "OPERATION_ELLIPSIS",
            setup: vec![ko("Umbra 캐시와 Vector 서버를 조사해.")],
            query: ko("뒤의 것부터 해."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Vector"),
        },
        Case {
            id: "E05",
            family: "OPERATION_ELLIPSIS",
            setup: vec![ko("Wolfram 워커를 수리해.")],
            query: ko("그거 먼저 해."),
            expected: Expected::Plan(PlanIntentIR::Repair, "Wolfram"),
        },
        Case {
            id: "E06",
            family: "OPERATION_ELLIPSIS",
            setup: vec![ko("Xenon 로그를 설명해.")],
            query: ko("그대로 진행해."),
            expected: Expected::Plan(PlanIntentIR::Explain, "Xenon"),
        },
        Case {
            id: "R01",
            family: "RETARGET_CORRECTION",
            setup: vec![en("Inspect the Yttrium cache and the Zircon queue.")],
            query: en("Not the first item; I meant the latter target."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Zircon"),
        },
        Case {
            id: "R02",
            family: "RETARGET_CORRECTION",
            setup: vec![en("Repair the Argent worker and the Bronze relay.")],
            query: en("Switch the repair target to the Bronze relay."),
            expected: Expected::Plan(PlanIntentIR::Repair, "Bronze"),
        },
        Case {
            id: "R03",
            family: "RETARGET_CORRECTION",
            setup: vec![en("Explain the Ceramic log and the Denim service.")],
            query: en("The explanation was meant for the second subject."),
            expected: Expected::Plan(PlanIntentIR::Explain, "Denim"),
        },
        Case {
            id: "R04",
            family: "RETARGET_CORRECTION",
            setup: vec![ko("Elm 캐시와 Fir 큐를 조사해.")],
            query: ko("앞의 것 말고 뒤 대상을 말한 거야."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Fir"),
        },
        Case {
            id: "R05",
            family: "RETARGET_CORRECTION",
            setup: vec![ko("Granite 워커와 Harbor 릴레이를 복구해.")],
            query: ko("복구 대상은 Harbor 릴레이로 바꿔."),
            expected: Expected::Plan(PlanIntentIR::Repair, "Harbor"),
        },
        Case {
            id: "R06",
            family: "RETARGET_CORRECTION",
            setup: vec![ko("Ivory 로그와 Jasper 서비스를 설명해.")],
            query: ko("설명 대상은 두 번째 항목이었어."),
            expected: Expected::Plan(PlanIntentIR::Explain, "Jasper"),
        },
        Case {
            id: "V01",
            family: "VERIFIED_RESULT_QUERY",
            setup: vec![en("Inspect the Krypton gateway.")],
            query: en("Do we have any observed findings for it yet?"),
            expected: Expected::ResultAbsence("Krypton"),
        },
        Case {
            id: "V02",
            family: "VERIFIED_RESULT_QUERY",
            setup: vec![en("Repair the Linen index.")],
            query: en("What repair result was actually verified?"),
            expected: Expected::ResultAbsence("Linen"),
        },
        Case {
            id: "V03",
            family: "VERIFIED_RESULT_QUERY",
            setup: vec![en("Diagnose the Marble pipeline.")],
            query: en("Are there confirmed findings, or only a plan?"),
            expected: Expected::ResultAbsence("Marble"),
        },
        Case {
            id: "V04",
            family: "VERIFIED_RESULT_QUERY",
            setup: vec![ko("Nectar 스케줄러를 조사해.")],
            query: ko("지금 확인된 관찰 결과가 있어?"),
            expected: Expected::ResultAbsence("Nectar"),
        },
        Case {
            id: "V05",
            family: "VERIFIED_RESULT_QUERY",
            setup: vec![ko("Opal 워커를 수리해.")],
            query: ko("실제로 검증된 수리 결과는 뭐야?"),
            expected: Expected::ResultAbsence("Opal"),
        },
        Case {
            id: "V06",
            family: "VERIFIED_RESULT_QUERY",
            setup: vec![ko("Pearl 서비스를 진단해.")],
            query: ko("확정된 결과가 있는 거야, 아직 계획뿐인 거야?"),
            expected: Expected::ResultAbsence("Pearl"),
        },
        Case {
            id: "A01",
            family: "AFFECT_REQUEST_CONTRAST",
            setup: vec![],
            query: en("The Resin outage has drained me today."),
            expected: Expected::Act(
                NativeResponseGoalIR::Acknowledge,
                NaturalResponseActIR::AffectSupport,
            ),
        },
        Case {
            id: "A02",
            family: "AFFECT_REQUEST_CONTRAST",
            setup: vec![],
            query: en("The Silver issue is draining me; help me work out the cause."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Silver"),
        },
        Case {
            id: "A03",
            family: "AFFECT_REQUEST_CONTRAST",
            setup: vec![],
            query: en("I am worn out from the Timber queue failing again."),
            expected: Expected::Act(
                NativeResponseGoalIR::Acknowledge,
                NaturalResponseActIR::AffectSupport,
            ),
        },
        Case {
            id: "A04",
            family: "AFFECT_REQUEST_CONTRAST",
            setup: vec![],
            query: ko("Umami 장애 때문에 오늘 완전히 지쳤어."),
            expected: Expected::Act(
                NativeResponseGoalIR::Acknowledge,
                NaturalResponseActIR::AffectSupport,
            ),
        },
        Case {
            id: "A05",
            family: "AFFECT_REQUEST_CONTRAST",
            setup: vec![],
            query: ko("Velvet 문제로 진이 빠지네. 원인을 찾는 걸 도와줘."),
            expected: Expected::Plan(PlanIntentIR::Investigate, "Velvet"),
        },
        Case {
            id: "A06",
            family: "AFFECT_REQUEST_CONTRAST",
            setup: vec![],
            query: ko("Willow 큐가 또 멈춰서 너무 지친다."),
            expected: Expected::Act(
                NativeResponseGoalIR::Acknowledge,
                NaturalResponseActIR::AffectSupport,
            ),
        },
    ]
}

fn request(id: &str, turn_index: u64, turn: Turn) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: format!("BOUNDARY-{id}"),
        turn_index,
        request_id: format!("BOUNDARY-{id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: turn.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(turn.language),
        context_tags: vec!["FRESH_PRODUCTIVE_RESPONSE_BOUNDARY".to_string()],
        max_plan_steps: 16,
    }
}

fn run(case: &Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, turn) in case.setup.iter().copied().enumerate() {
        let turn_index = u64::try_from(index + 1).expect("bounded setup");
        api.process_conversation_turn(&request(case.id, turn_index, turn))
            .unwrap_or_else(|error| panic!("setup {}: {error:?}", case.id));
    }
    let turn_index = u64::try_from(case.setup.len() + 1).expect("bounded query");
    let query = request(case.id, turn_index, case.query);
    let response = api
        .process_conversation_turn(&query)
        .unwrap_or_else(|error| panic!("query {}: {error:?}", case.id));
    let expected_pass = match case.expected {
        Expected::Plan(intent, target) => {
            let grounded_plan_matches =
                response.grounded_response.as_ref().is_some_and(|grounded| {
                    grounded.plan.intent == intent
                        && grounded
                            .understanding
                            .subject
                            .to_lowercase()
                            .contains(&target.to_lowercase())
                });
            response.native_language_circuit.response_goal == NativeResponseGoalIR::PlanActions
                && response.natural_realization.response_act == NaturalResponseActIR::PlanPreview
                && grounded_plan_matches
                && response
                    .native_language_circuit
                    .authoritative_single_live_goal()
                    .is_some_and(|goal| {
                        goal.intent == intent
                            && goal.subject.to_lowercase().contains(&target.to_lowercase())
                    })
        }
        Expected::ResultAbsence(target) => {
            response.native_language_circuit.response_goal
                == NativeResponseGoalIR::AnswerVerifiedResult
                && response.natural_realization.response_act == NaturalResponseActIR::ResultAbsence
                && response
                    .output
                    .text
                    .to_lowercase()
                    .contains(&target.to_lowercase())
        }
        Expected::Act(goal, act) => {
            response.native_language_circuit.response_goal == goal
                && response.natural_realization.response_act == act
        }
    };
    let safety_pass = response.validate_against(&query)
        && !response.six_axis_integration.semantic_authority
        && !response.six_axis_integration.language_can_execute
        && !response
            .language_cortex_integration
            .external_action_executed
        && response.output.unsupported_freeform_claims == 0;
    Row {
        id: case.id.to_string(),
        family: case.family.to_string(),
        pass: expected_pass && safety_pass,
        response_goal: format!("{:?}", response.native_language_circuit.response_goal),
        response_act: format!("{:?}", response.natural_realization.response_act),
        selected_goals: response
            .native_language_circuit
            .selected_live_goals
            .iter()
            .map(|goal| format!("{:?}:{}", goal.intent, goal.subject))
            .collect(),
        ambiguity_count: response
            .reference_resolution
            .ambiguous_reference_surfaces
            .len(),
        unresolved_count: response.native_language_circuit.unresolved.len()
            + response.pragmatic_interpretation.unresolved_bindings.len(),
        reference_ambiguities: response
            .reference_resolution
            .ambiguous_reference_surfaces
            .clone(),
        native_unresolved: response.native_language_circuit.unresolved.clone(),
        pragmatic_unresolved: response
            .pragmatic_interpretation
            .unresolved_bindings
            .clone(),
        output: response.output.text,
        safety_pass,
    }
}

fn main() {
    let rows = cases().iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let mut families = cases().iter().map(|case| case.family).collect::<Vec<_>>();
    families.sort_unstable();
    families.dedup();
    let family_scores = families
        .into_iter()
        .map(|family| FamilyScore {
            family: family.to_string(),
            passed: rows
                .iter()
                .filter(|row| row.family == family && row.pass)
                .count(),
            total: rows.iter().filter(|row| row.family == family).count(),
        })
        .collect();
    let report = Report {
        schema: "B_CORE_PRODUCTIVE_RESPONSE_BOUNDARY_REPORT_1",
        suite: "POST_V3_FRESH_PRODUCTIVE_BOUNDARY_1",
        frozen_v3_prompt_reuse: false,
        final_gpt_score_claimed: false,
        cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        pass_rate_basis_points: passed * 10_000 / rows.len(),
        family_scores,
        safety_violations: rows.iter().filter(|row| !row.safety_pass).count(),
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("serialize report")
    );
    if report.failed > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fresh_productive_boundary_cases_pass() {
        let rows = cases().iter().map(run).collect::<Vec<_>>();
        let failures = rows
            .iter()
            .filter(|row| !row.pass)
            .map(|row| format!("{}:{}", row.id, row.output))
            .collect::<Vec<_>>();
        assert!(failures.is_empty(), "failures={failures:#?}");
        assert_eq!(rows.len(), 36);
    }
}
