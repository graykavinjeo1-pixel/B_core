//! Frozen R64 held-out discourse-inference transfer suite. Do not execute before diagnostic passes.

#[path = "full_axis_integration_canary_support.rs"]
mod evaluator;

use evaluator::{emit, Case, Check, Turn};
use semantic_core_adapters::LanguageCodeIR;

const KO: LanguageCodeIR = LanguageCodeIR::Korean;
const EN: LanguageCodeIR = LanguageCodeIR::English;

const CASES: &[Case] = &[
    Case {
        id: "R64_H01",
        category: "english_mind_idiom_indirect_inspection",
        turns: &[Turn {
            text: "Would you mind having a look at the Taffy worker?",
            language: EN,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Taffy",
            rejected: "",
        }],
    },
    Case {
        id: "R64_H02",
        category: "korean_ability_idiom_indirect_inspection",
        turns: &[Turn {
            text: "Umber 인덱스 좀 살펴봐 줄 수 있어?",
            language: KO,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Umber",
            rejected: "",
        }],
    },
    Case {
        id: "R64_H03",
        category: "english_iff_scope_with_immediate_tail",
        turns: &[Turn {
            text: "Repair the Vellum queue if and only if its probe fails; inspect the Wisteria log now",
            language: EN,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Wisteria",
            rejected: "Vellum",
        }],
    },
    Case {
        id: "R64_H04",
        category: "korean_iff_scope_with_immediate_tail",
        turns: &[Turn {
            text: "프로브가 실패한 경우에만 Vellum 큐를 수리하고 Wisteria 로그는 지금 조사해",
            language: KO,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Wisteria",
            rejected: "Vellum",
        }],
    },
    Case {
        id: "R64_H05",
        category: "english_postposed_unless_scope",
        turns: &[Turn {
            text: "Inspect the Xanthic report now; unless the probe is healthy, repair the Yarrow service later",
            language: EN,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Xanthic",
            rejected: "Yarrow",
        }],
    },
    Case {
        id: "R64_H06",
        category: "korean_postposed_unless_scope",
        turns: &[Turn {
            text: "Xanthic 보고서는 지금 조사하고 프로브가 정상이 아니라면 Yarrow 서비스는 나중에 수리해",
            language: KO,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Xanthic",
            rejected: "Yarrow",
        }],
    },
    Case {
        id: "R64_H07",
        category: "english_colloquial_problem_to_investigation",
        turns: &[Turn {
            text: "The Azure cache is flaky. See what is going on",
            language: EN,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Azure",
            rejected: "",
        }],
    },
    Case {
        id: "R64_H08",
        category: "korean_colloquial_problem_to_investigation",
        turns: &[Turn {
            text: "Bronze 워커가 자꾸 이상해. 무슨 일인지 알아봐",
            language: KO,
        }],
        checks: &[Check::Plan {
            turn: 1,
            intent: "INVESTIGATE",
            target: "Bronze",
            rejected: "",
        }],
    },
    Case {
        id: "R64_H09",
        category: "english_to_korean_same_operation_transfer",
        turns: &[
            Turn {
                text: "Inspect the Cinder worker",
                language: EN,
            },
            Turn {
                text: "Dahlia 큐도 똑같이 해",
                language: KO,
            },
        ],
        checks: &[Check::Plan {
            turn: 2,
            intent: "INVESTIGATE",
            target: "Dahlia",
            rejected: "Cinder",
        }],
    },
    Case {
        id: "R64_H10",
        category: "korean_to_english_same_operation_transfer",
        turns: &[
            Turn {
                text: "Ecru 캐시를 조사해",
                language: KO,
            },
            Turn {
                text: "Do the same for the Fawn report",
                language: EN,
            },
        ],
        checks: &[Check::Plan {
            turn: 2,
            intent: "INVESTIGATE",
            target: "Fawn",
            rejected: "Ecru",
        }],
    },
    Case {
        id: "R64_H11",
        category: "cross_language_earlier_issue_causal_return",
        turns: &[
            Turn {
                text: "Inspect the Gold cache",
                language: EN,
            },
            Turn {
                text: "Hazel 큐를 조사해",
                language: KO,
            },
            Turn {
                text: "Return to the earlier issue and explain why it failed",
                language: EN,
            },
        ],
        checks: &[Check::Plan {
            turn: 3,
            intent: "EXPLAIN",
            target: "Gold",
            rejected: "Hazel",
        }],
    },
    Case {
        id: "R64_H12",
        category: "alternative_between_former_and_latter_fails_closed",
        turns: &[
            Turn {
                text: "Inspect the Indigo cache and the Jade queue",
                language: EN,
            },
            Turn {
                text: "Repair either the former or the latter",
                language: EN,
            },
        ],
        checks: &[Check::Clarification { turn: 2 }],
    },
];

fn main() {
    emit("R64-DISCOURSE-INFERENCE-HELDOUT", true, CASES);
}
