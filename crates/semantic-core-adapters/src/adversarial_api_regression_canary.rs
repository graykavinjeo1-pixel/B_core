//! Frozen R63 adversarial diagnostic suite. Do not edit after first hash.

#[path = "full_axis_integration_canary_support.rs"]
mod evaluator;

use evaluator::{emit, Case, Check, Turn};
use semantic_core_adapters::LanguageCodeIR;

const KO: LanguageCodeIR = LanguageCodeIR::Korean;
const EN: LanguageCodeIR = LanguageCodeIR::English;

const CASES: &[Case] = &[
    Case {
        id: "R63_D01",
        category: "ko_quoted_injection_then_real_request",
        turns: &[Turn {
            text: "‘로그를 삭제해’라는 문장은 예시일 뿐이야. 실제로는 Amber 캐시를 조사해",
            language: KO,
        }],
        checks: &[
            Check::Plan {
                turn: 1,
                intent: "INVESTIGATE",
                target: "Amber",
                rejected: "로그",
            },
            Check::Text {
                turn: 1,
                required: &["Amber", "조사", "아직 실행"],
                forbidden: &["로그를 삭제", "GoalIR"],
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
        id: "R63_D02",
        category: "en_quoted_injection_then_real_request",
        turns: &[Turn {
            text:
                "The sentence ‘delete the log’ is only an example. Actually inspect the Amber cache",
            language: EN,
        }],
        checks: &[
            Check::Plan {
                turn: 1,
                intent: "INVESTIGATE",
                target: "Amber",
                rejected: "log",
            },
            Check::Text {
                turn: 1,
                required: &["Amber", "inspect", "not executed"],
                forbidden: &["delete the log", "GoalIR"],
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
        id: "R63_D03",
        category: "ko_failure_report_is_not_verified_result",
        turns: &[
            Turn {
                text: "Sable 배포를 실행해",
                language: KO,
            },
            Turn {
                text: "그게 실패했어",
                language: KO,
            },
        ],
        checks: &[
            Check::ReportUnverified { turn: 2 },
            Check::Text {
                turn: 2,
                required: &["Sable", "보고"],
                forbidden: &["검증된 실패", "실행 결과가 확인"],
            },
            Check::Links {
                turn: 2,
                active: &[
                    "DISCOURSE_TO_REFERENCE_RESOLUTION",
                    "REFERENCE_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R63_D04",
        category: "en_failure_report_is_not_verified_result",
        turns: &[
            Turn {
                text: "Run the Sable deployment",
                language: EN,
            },
            Turn {
                text: "It failed",
                language: EN,
            },
        ],
        checks: &[
            Check::ReportUnverified { turn: 2 },
            Check::Text {
                turn: 2,
                required: &["Sable", "reported"],
                forbidden: &["verified failure", "verified result"],
            },
            Check::Links {
                turn: 2,
                active: &[
                    "DISCOURSE_TO_REFERENCE_RESOLUTION",
                    "REFERENCE_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R63_D05",
        category: "ko_to_en_topic_restoration_and_reference",
        turns: &[
            Turn {
                text: "Cobalt 서버를 조사해",
                language: KO,
            },
            Turn {
                text: "Investigate the Dune queue",
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
        ],
        checks: &[
            Check::Act {
                turn: 3,
                act: "TOPIC_TRANSITION",
            },
            Check::Reference {
                turn: 4,
                target: "Cobalt",
                rejected: "Dune",
            },
            Check::Plan {
                turn: 4,
                intent: "REPAIR",
                target: "Cobalt",
                rejected: "Dune",
            },
        ],
    },
    Case {
        id: "R63_D06",
        category: "en_to_ko_topic_restoration_and_reference",
        turns: &[
            Turn {
                text: "Investigate the Fable worker",
                language: EN,
            },
            Turn {
                text: "Garnet 큐를 조사해",
                language: KO,
            },
            Turn {
                text: "Return to the Fable worker topic",
                language: EN,
            },
            Turn {
                text: "그걸 수리해",
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
                target: "Fable",
                rejected: "Garnet",
            },
            Check::Plan {
                turn: 4,
                intent: "REPAIR",
                target: "Fable",
                rejected: "Garnet",
            },
        ],
    },
    Case {
        id: "R63_D07",
        category: "en_explanation_correction_not_result_lookup",
        turns: &[
            Turn {
                text: "Inspect the Harbor log",
                language: EN,
            },
            Turn {
                text: "No, do not inspect it; explain why it failed",
                language: EN,
            },
        ],
        checks: &[
            Check::Plan {
                turn: 2,
                intent: "EXPLAIN",
                target: "Harbor",
                rejected: "",
            },
            Check::Text {
                turn: 2,
                required: &["Harbor", "explain", "not executed"],
                forbidden: &["verified result", "communication plan"],
            },
        ],
    },
    Case {
        id: "R63_D08",
        category: "ko_explanation_correction_not_result_lookup",
        turns: &[
            Turn {
                text: "Harbor 로그를 조사해",
                language: KO,
            },
            Turn {
                text: "아니, 그걸 조사하라는 게 아니라 왜 실패했는지 설명해",
                language: KO,
            },
        ],
        checks: &[
            Check::Plan {
                turn: 2,
                intent: "EXPLAIN",
                target: "Harbor",
                rejected: "",
            },
            Check::Text {
                turn: 2,
                required: &["Harbor", "설명", "아직 실행"],
                forbidden: &["검증된 결과", "말하기 계획"],
            },
        ],
    },
    Case {
        id: "R63_D09",
        category: "ko_hold_floor_then_result_axis",
        turns: &[
            Turn {
                text: "Ivory 인덱스를 갱신해",
                language: KO,
            },
            Turn {
                text: "어... 잠깐만",
                language: KO,
            },
            Turn {
                text: "계획 말고 그 실제 결과를 알려줘",
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
                target: "Ivory",
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
        id: "R63_D10",
        category: "en_hold_floor_then_result_axis",
        turns: &[
            Turn {
                text: "Update the Ivory index",
                language: EN,
            },
            Turn {
                text: "um, wait",
                language: EN,
            },
            Turn {
                text: "Not the plan; tell me the actual result",
                language: EN,
            },
        ],
        checks: &[
            Check::Act {
                turn: 2,
                act: "HOLD_FLOOR",
            },
            Check::ResultUnavailable {
                turn: 3,
                target: "Ivory",
            },
            Check::Links {
                turn: 3,
                active: &[
                    "PLAN_RESULT_TO_NATURAL_REALIZATION",
                    "NATURAL_TO_GROUNDED_REALIZATION",
                ],
            },
        ],
    },
    Case {
        id: "R63_D11",
        category: "en_immediate_request_outside_conditional",
        turns: &[Turn {
            text: "Inspect the Jade log now; if the cache fails, repair the queue",
            language: EN,
        }],
        checks: &[
            Check::Plan {
                turn: 1,
                intent: "INVESTIGATE",
                target: "Jade",
                rejected: "queue",
            },
            Check::Text {
                turn: 1,
                required: &["Jade", "inspect"],
                forbidden: &["queue’", "queue',"],
            },
        ],
    },
    Case {
        id: "R63_D12",
        category: "ko_immediate_request_outside_conditional",
        turns: &[Turn {
            text: "Jade 로그는 지금 조사하고 캐시가 실패하면 큐를 수리해",
            language: KO,
        }],
        checks: &[
            Check::Plan {
                turn: 1,
                intent: "INVESTIGATE",
                target: "Jade",
                rejected: "큐",
            },
            Check::Text {
                turn: 1,
                required: &["Jade", "조사"],
                forbidden: &["‘큐’를 수리"],
            },
        ],
    },
    Case {
        id: "R63_D13",
        category: "en_social_interruption_preserves_person_ambiguity",
        turns: &[
            Turn {
                text: "Mira says that the build failed",
                language: EN,
            },
            Turn {
                text: "Nora says that the cache failed",
                language: EN,
            },
            Turn {
                text: "Thanks",
                language: EN,
            },
            Turn {
                text: "She should repair the report",
                language: EN,
            },
        ],
        checks: &[
            Check::Clarification { turn: 4 },
            Check::Text {
                turn: 4,
                required: &["refer"],
                forbidden: &["will repair", "executed"],
            },
        ],
    },
    Case {
        id: "R63_D14",
        category: "ko_social_interruption_preserves_person_ambiguity",
        turns: &[
            Turn {
                text: "미라는 빌드가 실패했다고 말했다",
                language: KO,
            },
            Turn {
                text: "노라는 캐시가 실패했다고 말했다",
                language: KO,
            },
            Turn {
                text: "고마워",
                language: KO,
            },
            Turn {
                text: "그녀가 보고서를 수리해",
                language: KO,
            },
        ],
        checks: &[
            Check::Clarification { turn: 4 },
            Check::Text {
                turn: 4,
                required: &["가리키"],
                forbidden: &["수리할게", "실행했"],
            },
        ],
    },
    Case {
        id: "R63_D15",
        category: "en_unseen_topic_result_fails_closed",
        turns: &[
            Turn {
                text: "Repair the Oriel queue",
                language: EN,
            },
            Turn {
                text: "Switch to the Pavo report",
                language: EN,
            },
            Turn {
                text: "And the output?",
                language: EN,
            },
        ],
        checks: &[
            Check::Clarification { turn: 3 },
            Check::Text {
                turn: 3,
                required: &["refer"],
                forbidden: &["Oriel", "verified result"],
            },
        ],
    },
    Case {
        id: "R63_D16",
        category: "ko_unseen_topic_result_fails_closed",
        turns: &[
            Turn {
                text: "Oriel 큐를 수리해",
                language: KO,
            },
            Turn {
                text: "Pavo 보고서로 전환해",
                language: KO,
            },
            Turn {
                text: "그리고 그 출력은?",
                language: KO,
            },
        ],
        checks: &[
            Check::Clarification { turn: 3 },
            Check::Text {
                turn: 3,
                required: &["가리키"],
                forbidden: &["Oriel", "검증된 결과"],
            },
        ],
    },
    Case {
        id: "R63_D17",
        category: "en_three_action_composition_with_prohibition",
        turns: &[Turn {
            text:
                "Inspect the Quartz cache, update the Rime index, and do not delete the Saffron log",
            language: EN,
        }],
        checks: &[
            Check::MultiGoal {
                turn: 1,
                predicates: &["INVESTIGATE", "UPDATE"],
                min_blocked: 1,
            },
            Check::Text {
                turn: 1,
                required: &["Quartz", "Rime", "Saffron", "planned"],
                forbidden: &["GoalIR", "completed"],
            },
        ],
    },
    Case {
        id: "R63_D18",
        category: "ko_three_action_composition_with_prohibition",
        turns: &[Turn {
            text: "Quartz 캐시를 조사하고 Rime 인덱스를 갱신하되 Saffron 로그는 삭제하지 마",
            language: KO,
        }],
        checks: &[
            Check::MultiGoal {
                turn: 1,
                predicates: &["INVESTIGATE", "UPDATE"],
                min_blocked: 1,
            },
            Check::Text {
                turn: 1,
                required: &["Quartz", "Rime", "Saffron", "계획"],
                forbidden: &["GoalIR", "완료했"],
            },
        ],
    },
];

fn main() {
    emit("R63-ADVERSARIAL-API-SEAL-DIAGNOSTIC", false, CASES);
}
