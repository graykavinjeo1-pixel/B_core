//! Frozen R62 full-axis integration diagnostic suite.

mod full_axis_integration_canary_support;

use full_axis_integration_canary_support::{emit, Case, Check, Turn};
use semantic_core_adapters::LanguageCodeIR;

const KO: LanguageCodeIR = LanguageCodeIR::Korean;
const EN: LanguageCodeIR = LanguageCodeIR::English;

const CASES: &[Case] = &[
    Case {
        id: "R62_D01",
        category: "ko_topic_reference_plan_result_realization",
        turns: &[
            Turn {
                text: "Aurora 서버 캐시를 조사해",
                language: KO,
            },
            Turn {
                text: "Beryl 백업 큐를 조사해",
                language: KO,
            },
            Turn {
                text: "Aurora 서버 이야기로 돌아가자",
                language: KO,
            },
            Turn {
                text: "그걸 수리해",
                language: KO,
            },
            Turn {
                text: "그 결과는 어떻게 됐어?",
                language: KO,
            },
        ],
        checks: &[
            Check::Act {
                turn: 3,
                act: "TOPIC_TRANSITION",
            },
            Check::Reference {
                turn: 4,
                target: "서버",
                rejected: "백업",
            },
            Check::Plan {
                turn: 4,
                intent: "REPAIR",
                target: "서버",
                rejected: "백업",
            },
            Check::Act {
                turn: 5,
                act: "RESULT_ABSENCE",
            },
            Check::ResultUnavailable {
                turn: 5,
                target: "서버",
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
        id: "R62_D02",
        category: "en_topic_reference_plan_result_realization",
        turns: &[
            Turn {
                text: "Investigate the Aurora server cache",
                language: EN,
            },
            Turn {
                text: "Investigate the Beryl backup queue",
                language: EN,
            },
            Turn {
                text: "Return to the Aurora server topic",
                language: EN,
            },
            Turn {
                text: "Repair it",
                language: EN,
            },
            Turn {
                text: "What happened to that result?",
                language: EN,
            },
        ],
        checks: &[
            Check::Act {
                turn: 3,
                act: "TOPIC_TRANSITION",
            },
            Check::Reference {
                turn: 4,
                target: "server",
                rejected: "backup",
            },
            Check::Plan {
                turn: 4,
                intent: "REPAIR",
                target: "server",
                rejected: "backup",
            },
            Check::Act {
                turn: 5,
                act: "RESULT_ABSENCE",
            },
            Check::ResultUnavailable {
                turn: 5,
                target: "server",
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
        id: "R62_D03",
        category: "ko_negation_parallel_ellipsis_natural_plan",
        turns: &[
            Turn {
                text: "로그를 분석하되 삭제하지 마",
                language: KO,
            },
            Turn {
                text: "백업도 그렇게 해",
                language: KO,
            },
        ],
        checks: &[
            Check::MultiGoal {
                turn: 1,
                predicates: &["INVESTIGATE"],
                min_blocked: 1,
            },
            Check::Plan {
                turn: 2,
                intent: "INVESTIGATE",
                target: "백업",
                rejected: "로그",
            },
            Check::Text {
                turn: 2,
                required: &["백업", "아직 실행"],
                forbidden: &["DELETE", "GoalIR"],
            },
            Check::Links {
                turn: 2,
                active: &[
                    "REFERENCE_TO_PRAGMATIC_INTENT",
                    "PRAGMATIC_INTENT_TO_PLAN",
                    "REFERENCE_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_D04",
        category: "en_negation_parallel_ellipsis_natural_plan",
        turns: &[
            Turn {
                text: "Analyze the log but do not delete it",
                language: EN,
            },
            Turn {
                text: "Do the same for the backup",
                language: EN,
            },
        ],
        checks: &[
            Check::MultiGoal {
                turn: 1,
                predicates: &["INVESTIGATE"],
                min_blocked: 1,
            },
            Check::Plan {
                turn: 2,
                intent: "INVESTIGATE",
                target: "backup",
                rejected: "log",
            },
            Check::Text {
                turn: 2,
                required: &["backup", "not executed"],
                forbidden: &["DELETE", "GoalIR"],
            },
            Check::Links {
                turn: 2,
                active: &[
                    "REFERENCE_TO_PRAGMATIC_INTENT",
                    "PRAGMATIC_INTENT_TO_PLAN",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_D05",
        category: "ko_feedback_correction_request",
        turns: &[
            Turn {
                text: "캐시를 조사해",
                language: KO,
            },
            Turn {
                text: "아니, 방금 답은 핵심을 놓쳤어. 큐를 수리해",
                language: KO,
            },
        ],
        checks: &[
            Check::Plan {
                turn: 2,
                intent: "REPAIR",
                target: "큐",
                rejected: "캐시",
            },
            Check::Text {
                turn: 2,
                required: &["핵심", "큐", "수리", "아직 실행"],
                forbidden: &["사용자의 피드백", "선택 행동"],
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
        id: "R62_D06",
        category: "en_feedback_correction_request",
        turns: &[
            Turn {
                text: "Investigate the cache",
                language: EN,
            },
            Turn {
                text: "No, that missed the main point. Repair the queue",
                language: EN,
            },
        ],
        checks: &[
            Check::Plan {
                turn: 2,
                intent: "REPAIR",
                target: "queue",
                rejected: "cache",
            },
            Check::Text {
                turn: 2,
                required: &["missed", "queue", "repair", "not executed"],
                forbidden: &["user feedback", "selected action"],
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
        id: "R62_D07",
        category: "ko_language_report_lifecycle_reference",
        turns: &[
            Turn {
                text: "Indigo 배포를 실행해",
                language: KO,
            },
            Turn {
                text: "그건 완료됐어",
                language: KO,
            },
            Turn {
                text: "그 결과가 검증됐어?",
                language: KO,
            },
        ],
        checks: &[
            Check::ReportUnverified { turn: 2 },
            Check::ReportUnverified { turn: 3 },
            Check::ResultUnavailable {
                turn: 3,
                target: "Indigo",
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
        id: "R62_D08",
        category: "en_language_report_lifecycle_reference",
        turns: &[
            Turn {
                text: "Run the Indigo deployment",
                language: EN,
            },
            Turn {
                text: "That finished",
                language: EN,
            },
            Turn {
                text: "Was that result verified?",
                language: EN,
            },
        ],
        checks: &[
            Check::ReportUnverified { turn: 2 },
            Check::ReportUnverified { turn: 3 },
            Check::ResultUnavailable {
                turn: 3,
                target: "Indigo",
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
        id: "R62_D09",
        category: "ko_person_ambiguity_fails_closed_naturally",
        turns: &[
            Turn {
                text: "마루는 빌드가 실패했다고 말했다",
                language: KO,
            },
            Turn {
                text: "아라는 캐시가 실패했다고 말했다",
                language: KO,
            },
            Turn {
                text: "그녀가 보고서를 수정해",
                language: KO,
            },
        ],
        checks: &[
            Check::Clarification { turn: 3 },
            Check::Text {
                turn: 3,
                required: &["가리키"],
                forbidden: &["수정할게", "실행했"],
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
        id: "R62_D10",
        category: "en_person_ambiguity_fails_closed_naturally",
        turns: &[
            Turn {
                text: "Quinn says that the build failed",
                language: EN,
            },
            Turn {
                text: "Rowan says that the cache failed",
                language: EN,
            },
            Turn {
                text: "She should revise the report",
                language: EN,
            },
        ],
        checks: &[
            Check::Clarification { turn: 3 },
            Check::Text {
                turn: 3,
                required: &["refer"],
                forbidden: &["will revise", "executed"],
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
        id: "R62_D11",
        category: "ko_hold_floor_preserves_task_for_continuation",
        turns: &[
            Turn {
                text: "Quartz 캐시를 수리해",
                language: KO,
            },
            Turn {
                text: "음...",
                language: KO,
            },
            Turn {
                text: "그 작업 계속해",
                language: KO,
            },
        ],
        checks: &[
            Check::Act {
                turn: 2,
                act: "HOLD_FLOOR",
            },
            Check::Plan {
                turn: 3,
                intent: "EXECUTE",
                target: "Quartz",
                rejected: "",
            },
            Check::Links {
                turn: 3,
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
        id: "R62_D12",
        category: "en_hold_floor_preserves_task_for_continuation",
        turns: &[
            Turn {
                text: "Repair the Quartz cache",
                language: EN,
            },
            Turn {
                text: "uh...",
                language: EN,
            },
            Turn {
                text: "Keep doing that work",
                language: EN,
            },
        ],
        checks: &[
            Check::Act {
                turn: 2,
                act: "HOLD_FLOOR",
            },
            Check::Plan {
                turn: 3,
                intent: "EXECUTE",
                target: "Quartz",
                rejected: "",
            },
            Check::Links {
                turn: 3,
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
        id: "R62_D13",
        category: "ko_response_axis_correction_preserves_action",
        turns: &[
            Turn {
                text: "Opal 작업을 조사해",
                language: KO,
            },
            Turn {
                text: "계획 말고 실제 실행 결과만 말해",
                language: KO,
            },
        ],
        checks: &[
            Check::Act {
                turn: 2,
                act: "PLAN_RESULT_STATUS",
            },
            Check::ResultUnavailable {
                turn: 2,
                target: "Opal",
            },
            Check::Text {
                turn: 2,
                required: &["실행 결과", "계획"],
                forbidden: &["말하기 계획", "COMMUNICATE"],
            },
            Check::Links {
                turn: 2,
                active: &[
                    "PLAN_RESULT_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_D14",
        category: "en_response_axis_correction_preserves_action",
        turns: &[
            Turn {
                text: "Investigate the Opal job",
                language: EN,
            },
            Turn {
                text: "Tell me only the actual execution result, not the plan",
                language: EN,
            },
        ],
        checks: &[
            Check::Act {
                turn: 2,
                act: "PLAN_RESULT_STATUS",
            },
            Check::ResultUnavailable {
                turn: 2,
                target: "Opal",
            },
            Check::Text {
                turn: 2,
                required: &["execution result", "plan"],
                forbidden: &["communication plan", "COMMUNICATE"],
            },
            Check::Links {
                turn: 2,
                active: &[
                    "PLAN_RESULT_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R62_D15",
        category: "ko_composed_goals_prohibition_natural_realization",
        turns: &[Turn {
            text: "캐시를 조사하고 큐를 수리하되 로그는 삭제하지 마",
            language: KO,
        }],
        checks: &[
            Check::MultiGoal {
                turn: 1,
                predicates: &["INVESTIGATE", "REPAIR"],
                min_blocked: 1,
            },
            Check::Text {
                turn: 1,
                required: &["캐시", "큐", "계획", "검증"],
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
    Case {
        id: "R62_D16",
        category: "en_composed_goals_prohibition_natural_realization",
        turns: &[Turn {
            text: "Investigate the cache and repair the queue, but do not delete the log",
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
                required: &["cache", "queue", "planned", "verification"],
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
    emit("R62-FULL-AXIS-INTEGRATION-DIAGNOSTIC", false, CASES);
}
