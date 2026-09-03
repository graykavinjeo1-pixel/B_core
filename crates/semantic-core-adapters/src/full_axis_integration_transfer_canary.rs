//! Frozen R62 first-exposure full-axis integration transfer suite.

mod full_axis_integration_canary_support;

use full_axis_integration_canary_support::{emit, Case, Check, Turn};
use semantic_core_adapters::LanguageCodeIR;

const KO: LanguageCodeIR = LanguageCodeIR::Korean;
const EN: LanguageCodeIR = LanguageCodeIR::English;

const CASES: &[Case] = &[
    Case {
        id: "R62_H01",
        category: "cross_language_topic_reference_result",
        turns: &[
            Turn {
                text: "Cobalt 서버를 조사해",
                language: KO,
            },
            Turn {
                text: "Investigate the Amber queue",
                language: EN,
            },
            Turn {
                text: "Cobalt 서버 이야기로 돌아가자",
                language: KO,
            },
            Turn {
                text: "Repair it",
                language: EN,
            },
            Turn {
                text: "그 결과는?",
                language: KO,
            },
        ],
        checks: &[
            Check::Reference {
                turn: 4,
                target: "Cobalt",
                rejected: "Amber",
            },
            Check::Plan {
                turn: 4,
                intent: "REPAIR",
                target: "Cobalt",
                rejected: "Amber",
            },
            Check::ResultUnavailable {
                turn: 5,
                target: "Cobalt",
            },
            Check::Links {
                turn: 4,
                active: &[
                    "DISCOURSE_TO_REFERENCE_RESOLUTION",
                    "REFERENCE_TO_PRAGMATIC_INTENT",
                    "PRAGMATIC_INTENT_TO_PLAN",
                    "REFERENCE_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_H02",
        category: "reverse_cross_language_topic_reference_result",
        turns: &[
            Turn {
                text: "Investigate the Garnet worker",
                language: EN,
            },
            Turn {
                text: "Lime 백업을 조사해",
                language: KO,
            },
            Turn {
                text: "Return to the Garnet worker topic",
                language: EN,
            },
            Turn {
                text: "그걸 수리해",
                language: KO,
            },
            Turn {
                text: "Was that result verified?",
                language: EN,
            },
        ],
        checks: &[
            Check::Reference {
                turn: 4,
                target: "Garnet",
                rejected: "Lime",
            },
            Check::Plan {
                turn: 4,
                intent: "REPAIR",
                target: "Garnet",
                rejected: "Lime",
            },
            Check::ResultUnavailable {
                turn: 5,
                target: "Garnet",
            },
            Check::Links {
                turn: 5,
                active: &[
                    "PLAN_RESULT_TO_NATURAL_REALIZATION",
                    "REFERENCE_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_H03",
        category: "english_local_descriptor_reference",
        turns: &[
            Turn {
                text: "The Topaz service was reviewed by Uma",
                language: EN,
            },
            Turn {
                text: "Inspect that service again",
                language: EN,
            },
        ],
        checks: &[
            Check::Reference {
                turn: 2,
                target: "Topaz",
                rejected: "Uma",
            },
            Check::Plan {
                turn: 2,
                intent: "INVESTIGATE",
                target: "Topaz",
                rejected: "Uma",
            },
            Check::Links {
                turn: 2,
                active: &[
                    "DISCOURSE_TO_REFERENCE_RESOLUTION",
                    "REFERENCE_TO_PRAGMATIC_INTENT",
                    "PRAGMATIC_INTENT_TO_PLAN",
                    "REFERENCE_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_H04",
        category: "korean_local_descriptor_reference",
        turns: &[
            Turn {
                text: "Sage 서비스를 확인해",
                language: KO,
            },
            Turn {
                text: "그 서비스를 다시 조사해",
                language: KO,
            },
        ],
        checks: &[
            Check::Reference {
                turn: 2,
                target: "Sage",
                rejected: "",
            },
            Check::Plan {
                turn: 2,
                intent: "INVESTIGATE",
                target: "Sage",
                rejected: "",
            },
            Check::Links {
                turn: 2,
                active: &[
                    "DISCOURSE_TO_REFERENCE_RESOLUTION",
                    "REFERENCE_TO_PRAGMATIC_INTENT",
                    "PRAGMATIC_INTENT_TO_PLAN",
                    "REFERENCE_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_H05",
        category: "korean_feedback_retarget_proper_label",
        turns: &[
            Turn {
                text: "Ruby 캐시를 조사해",
                language: KO,
            },
            Turn {
                text: "그게 아니라 핵심은 Onyx 큐야. Onyx 큐를 수리해",
                language: KO,
            },
        ],
        checks: &[
            Check::Plan {
                turn: 2,
                intent: "REPAIR",
                target: "Onyx",
                rejected: "Ruby",
            },
            Check::Text {
                turn: 2,
                required: &["Onyx", "핵심", "수리", "아직 실행"],
                forbidden: &["선택 행동", "GoalIR"],
            },
            Check::Links {
                turn: 2,
                active: &[
                    "GRAMMATICAL_COMPOSITION_TO_PRAGMATIC_INTENT",
                    "PRAGMATIC_INTENT_TO_PLAN",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_H06",
        category: "english_feedback_retarget_proper_label",
        turns: &[
            Turn {
                text: "Investigate the Ruby cache",
                language: EN,
            },
            Turn {
                text: "That missed the key point. Repair the Onyx queue",
                language: EN,
            },
        ],
        checks: &[
            Check::Plan {
                turn: 2,
                intent: "REPAIR",
                target: "Onyx",
                rejected: "Ruby",
            },
            Check::Text {
                turn: 2,
                required: &["Onyx", "missed", "repair", "not executed"],
                forbidden: &["selected action", "GoalIR"],
            },
            Check::Links {
                turn: 2,
                active: &[
                    "GRAMMATICAL_COMPOSITION_TO_PRAGMATIC_INTENT",
                    "PRAGMATIC_INTENT_TO_PLAN",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_H07",
        category: "reported_completion_does_not_verify_multi_action_result",
        turns: &[
            Turn {
                text: "Investigate the Azure cache and repair the Azure queue",
                language: EN,
            },
            Turn {
                text: "They both finished",
                language: EN,
            },
            Turn {
                text: "Are those results verified?",
                language: EN,
            },
        ],
        checks: &[
            Check::ReportUnverified { turn: 2 },
            Check::ReportUnverified { turn: 3 },
            Check::Links {
                turn: 3,
                active: &[
                    "PLAN_RESULT_TO_NATURAL_REALIZATION",
                    "REFERENCE_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_H08",
        category: "heldout_person_ambiguity",
        turns: &[
            Turn {
                text: "Ophelia says that the test is blocked",
                language: EN,
            },
            Turn {
                text: "Yarrow says that the worker is ready",
                language: EN,
            },
            Turn {
                text: "She should repair the report",
                language: EN,
            },
        ],
        checks: &[
            Check::Clarification { turn: 3 },
            Check::Text {
                turn: 3,
                required: &["refer"],
                forbidden: &["will repair", "executed"],
            },
            Check::Links {
                turn: 3,
                active: &[
                    "DISCOURSE_TO_REFERENCE_RESOLUTION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_H09",
        category: "hold_floor_then_result_axis_correction",
        turns: &[
            Turn {
                text: "Cedar 인덱스를 갱신해",
                language: KO,
            },
            Turn {
                text: "어... 잠깐",
                language: KO,
            },
            Turn {
                text: "계획이 아니라 그 실제 결과를 말해",
                language: KO,
            },
        ],
        checks: &[
            Check::Act {
                turn: 2,
                act: "HOLD_FLOOR",
            },
            Check::ResultUnavailable {
                turn: 3,
                target: "Cedar",
            },
            Check::Links {
                turn: 3,
                active: &[
                    "PLAN_RESULT_TO_NATURAL_REALIZATION",
                    "REFERENCE_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_H10",
        category: "heldout_composed_plan_and_prohibition",
        turns: &[Turn {
            text:
                "Inspect the Violet worker, repair the Copper queue, and do not remove the Jade log",
            language: EN,
        }],
        checks: &[
            Check::MultiGoal {
                turn: 1,
                predicates: &["INVESTIGATE", "REPAIR"],
                min_blocked: 1,
            },
            Check::Text {
                turn: 1,
                required: &["Violet", "Copper", "planned", "verification"],
                forbidden: &["compositional_goal_graph", "GoalIR"],
            },
            Check::Links {
                turn: 1,
                active: &[
                    "GRAMMATICAL_COMPOSITION_TO_PRAGMATIC_INTENT",
                    "PRAGMATIC_INTENT_TO_PLAN",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
];

fn main() {
    emit("R62-FULL-AXIS-INTEGRATION-HELDOUT", true, CASES);
}
