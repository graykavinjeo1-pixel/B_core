//! Frozen R50 held-out suite, authored only after the diagnostic reached 12/12.

mod language_cortex_integration_canary_support;

use language_cortex_integration_canary_support::{emit, Case, Turn};
use semantic_core_adapters::ConversationTurnDispositionIR::{BackchannelOnly, Grounded};
use semantic_core_adapters::LanguageCodeIR::{English, Korean};

const CASES: &[Case] = &[
    Case {
        id: "R50H_01",
        category: "same_turn_korean_deixis",
        turns: &[Turn {
            text: "보고서를 열고 그걸 검사해",
            language: Korean,
        }],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50H_02",
        category: "cross_turn_english_reference",
        turns: &[
            Turn {
                text: "Open the report.",
                language: English,
            },
            Turn {
                text: "Check it.",
                language: English,
            },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50H_03",
        category: "fresh_korean_compound_guard",
        turns: &[Turn {
            text: "큐를 검사하고 비었거나 손상됐으면 복구해",
            language: Korean,
        }],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50H_04",
        category: "fresh_english_program_rebinding",
        turns: &[
            Turn {
                text: "Inspect the queue and if the queue is empty, repair the queue.",
                language: English,
            },
            Turn {
                text: "Apply that procedure to the cache as well.",
                language: English,
            },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50H_05",
        category: "korean_report_boundary",
        turns: &[
            Turn {
                text: "인덱스를 검사해",
                language: Korean,
            },
            Turn {
                text: "내가 끝냈어",
                language: Korean,
            },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50H_06",
        category: "english_verified_result_absence",
        turns: &[
            Turn {
                text: "Inspect the worker.",
                language: English,
            },
            Turn {
                text: "What is the verified result?",
                language: English,
            },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50H_07",
        category: "english_qud_answer",
        turns: &[
            Turn {
                text: "Should we inspect the cache or the index first?",
                language: English,
            },
            Turn {
                text: "The index.",
                language: English,
            },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50H_08",
        category: "english_backchannel_binding",
        turns: &[
            Turn {
                text: "Inspect the document.",
                language: English,
            },
            Turn {
                text: "Okay.",
                language: English,
            },
        ],
        expected_disposition: BackchannelOnly,
    },
];

fn main() {
    emit("R50_LANGUAGE_CORTEX_INTEGRATION_HELDOUT", CASES);
}
