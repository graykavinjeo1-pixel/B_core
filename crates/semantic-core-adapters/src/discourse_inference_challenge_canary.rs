//! Frozen R64 discourse-inference diagnostic suite. Do not edit after first hash.

#[path = "full_axis_integration_canary_support.rs"]
mod evaluator;

use evaluator::{emit, Case, Check, Turn};
use semantic_core_adapters::LanguageCodeIR;

const KO: LanguageCodeIR = LanguageCodeIR::Korean;
const EN: LanguageCodeIR = LanguageCodeIR::English;

const CASES: &[Case] = &[
    Case {
        id: "R64_D01",
        category: "english_polite_indirect_inspection_request",
        turns: &[Turn {
            text: "Could you take a look at the Aster cache?",
            language: EN,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Aster",
            rejected: "",
        }],
    },
    Case {
        id: "R64_D02",
        category: "korean_polite_indirect_inspection_request",
        turns: &[Turn {
            text: "Aster 캐시 좀 봐줄래?",
            language: KO,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Aster",
            rejected: "",
        }],
    },
    Case {
        id: "R64_D03",
        category: "english_only_if_scope_preserves_immediate_action",
        turns: &[Turn {
            text: "Inspect the Birch log now, but repair the Cedar queue only if the cache is stale",
            language: EN,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Birch",
            rejected: "Cedar",
        }],
    },
    Case {
        id: "R64_D04",
        category: "korean_only_when_scope_preserves_immediate_action",
        turns: &[Turn {
            text: "Birch 로그는 지금 조사하되 캐시가 오래됐을 때만 Cedar 큐를 수리해",
            language: KO,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Birch",
            rejected: "Cedar",
        }],
    },
    Case {
        id: "R64_D05",
        category: "english_unless_scope_preserves_separate_immediate_action",
        turns: &[Turn {
            text: "Unless the Dune service is healthy, repair the Ember worker; inspect the Flint report now",
            language: EN,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Flint",
            rejected: "Ember",
        }],
    },
    Case {
        id: "R64_D06",
        category: "korean_negative_condition_preserves_separate_immediate_action",
        turns: &[Turn {
            text: "Dune 서비스가 정상이 아닌 경우에만 Ember 워커를 수리하고 지금은 Flint 보고서를 조사해",
            language: KO,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Flint",
            rejected: "Ember",
        }],
    },
    Case {
        id: "R64_D07",
        category: "english_concessive_prohibition_with_live_explanation",
        turns: &[Turn {
            text: "Even if the Garnet cache failed, do not delete it; explain why it failed",
            language: EN,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "EXPLAIN",
            target: "Garnet",
            rejected: "delete",
        }],
    },
    Case {
        id: "R64_D08",
        category: "korean_concessive_prohibition_with_live_explanation",
        turns: &[Turn {
            text: "Garnet 캐시가 실패했더라도 그걸 삭제하지 말고 왜 실패했는지 설명해",
            language: KO,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "EXPLAIN",
            target: "Garnet",
            rejected: "삭제",
        }],
    },
    Case {
        id: "R64_D09",
        category: "english_same_turn_contrastive_retarget",
        turns: &[Turn {
            text: "Not the Ivory index—the Juniper queue. Repair that one",
            language: EN,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "REPAIR",
            target: "Juniper",
            rejected: "Ivory",
        }],
    },
    Case {
        id: "R64_D10",
        category: "korean_same_turn_contrastive_retarget",
        turns: &[Turn {
            text: "Ivory 인덱스 말고 Juniper 큐야. 그걸 수리해",
            language: KO,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "REPAIR",
            target: "Juniper",
            rejected: "Ivory",
        }],
    },
    Case {
        id: "R64_D11",
        category: "english_cross_turn_same_operation_ellipsis",
        turns: &[
            Turn {
                text: "Inspect the Kestrel worker",
                language: EN,
            },
            Turn {
                text: "Do the same to the Linen queue",
                language: EN,
            },
        ],
        checks: &[Check::Plan {
            turn: 2,
            intent: "INVESTIGATE",
            target: "Linen",
            rejected: "Kestrel",
        }],
    },
    Case {
        id: "R64_D12",
        category: "korean_cross_turn_same_operation_ellipsis",
        turns: &[
            Turn {
                text: "Kestrel 워커를 조사해",
                language: KO,
            },
            Turn {
                text: "Linen 큐에도 똑같이 해",
                language: KO,
            },
        ],
        checks: &[Check::Plan {
            turn: 2,
            intent: "INVESTIGATE",
            target: "Linen",
            rejected: "Kestrel",
        }],
    },
    Case {
        id: "R64_D13",
        category: "english_problem_disclosure_plus_causal_investigation",
        turns: &[Turn {
            text: "The Mallow service keeps timing out. Find out why",
            language: EN,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Mallow",
            rejected: "",
        }],
    },
    Case {
        id: "R64_D14",
        category: "korean_problem_disclosure_plus_causal_investigation",
        turns: &[Turn {
            text: "Navy 서비스가 계속 시간 초과돼. 원인을 확인해 줘",
            language: KO,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Navy",
            rejected: "",
        }],
    },
    Case {
        id: "R64_D15",
        category: "english_report_rejection_then_verified_result_question",
        turns: &[
            Turn {
                text: "Run the Ocher migration",
                language: EN,
            },
            Turn {
                text: "Someone said it finished",
                language: EN,
            },
            Turn {
                text: "I do not need the claim. Tell me whether the actual result was verified",
                language: EN,
            },
        ],
        checks: &[
            Check::ReportUnverified { turn: 2 },
            Check::ResultUnavailable {
                turn: 3,
                target: "Ocher",
            },
        ],
    },
    Case {
        id: "R64_D16",
        category: "korean_report_rejection_then_verified_result_question",
        turns: &[
            Turn {
                text: "Ocher 마이그레이션을 실행해",
                language: KO,
            },
            Turn {
                text: "누가 그게 끝났다고 했어",
                language: KO,
            },
            Turn {
                text: "그 주장은 필요 없어. 실제 결과가 검증됐는지 알려줘",
                language: KO,
            },
        ],
        checks: &[
            Check::ReportUnverified { turn: 2 },
            Check::ResultUnavailable {
                turn: 3,
                target: "Ocher",
            },
        ],
    },
    Case {
        id: "R64_D17",
        category: "english_ordinal_issue_return_then_causal_goal",
        turns: &[
            Turn {
                text: "Inspect the Parchment cache",
                language: EN,
            },
            Turn {
                text: "Inspect the Quartz queue",
                language: EN,
            },
            Turn {
                text: "Go back to the first issue and explain why it failed",
                language: EN,
            },
        ],
        checks: &[Check::Plan {
            turn: 3,
            intent: "EXPLAIN",
            target: "Parchment",
            rejected: "Quartz",
        }],
    },
    Case {
        id: "R64_D18",
        category: "korean_ordinal_issue_return_then_causal_goal",
        turns: &[
            Turn {
                text: "Parchment 캐시를 조사해",
                language: KO,
            },
            Turn {
                text: "Quartz 큐를 조사해",
                language: KO,
            },
            Turn {
                text: "첫 번째 문제로 돌아가서 왜 실패했는지 설명해",
                language: KO,
            },
        ],
        checks: &[Check::Plan {
            turn: 3,
            intent: "EXPLAIN",
            target: "Parchment",
            rejected: "Quartz",
        }],
    },
    Case {
        id: "R64_D19",
        category: "english_coordinated_set_with_latter_only_repair",
        turns: &[Turn {
            text: "Inspect the Rose cache and the Sienna queue, but repair only the latter",
            language: EN,
        }],
        checks: &[
            Check::MultiGoal {
                turn: 1,
                predicates: &["INVESTIGATE", "REPAIR"],
                min_blocked: 0,
            },
            Check::Text {
                turn: 1,
                required: &["Rose", "Sienna"],
                forbidden: &["GoalIR", "completed"],
            },
        ],
    },
    Case {
        id: "R64_D20",
        category: "korean_coordinated_set_with_latter_only_repair",
        turns: &[Turn {
            text: "Rose 캐시와 Sienna 큐를 조사하되 후자만 수리해",
            language: KO,
        }],
        checks: &[
            Check::MultiGoal {
                turn: 1,
                predicates: &["INVESTIGATE", "REPAIR"],
                min_blocked: 0,
            },
            Check::Text {
                turn: 1,
                required: &["Rose", "Sienna"],
                forbidden: &["GoalIR", "완료했"],
            },
        ],
    },
];

fn main() {
    emit("R64-DISCOURSE-INFERENCE-DIAGNOSTIC", false, CASES);
}
