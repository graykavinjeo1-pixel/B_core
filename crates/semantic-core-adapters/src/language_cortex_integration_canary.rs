//! Frozen R50 diagnostic suite for the final Language Cortex response contract.

mod language_cortex_integration_canary_support;

use language_cortex_integration_canary_support::{emit, Case, Turn};
use semantic_core_adapters::ConversationTurnDispositionIR::{ClarificationRequired, Grounded};
use semantic_core_adapters::LanguageCodeIR::{English, Korean};

const CASES: &[Case] = &[
    Case {
        id: "R50_01",
        category: "korean_noisy_grounding",
        turns: &[Turn { text: "음... 캐시를 검사하고 오래됐으면 수리해", language: Korean }],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50_02",
        category: "english_compound_guard",
        turns: &[Turn { text: "Inspect the cache and if it is stale or damaged, repair it.", language: English }],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50_03",
        category: "cross_language_program_rebinding",
        turns: &[
            Turn { text: "Inspect the cache and if the cache is stale, repair the cache.", language: English },
            Turn { text: "인덱스도 같은 절차로 해", language: Korean },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50_04",
        category: "deixis_and_ellipsis",
        turns: &[
            Turn { text: "파일을 열어", language: Korean },
            Turn { text: "그걸 확인해", language: Korean },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50_05",
        category: "qud_adjacency",
        turns: &[
            Turn { text: "캐시와 인덱스 중 무엇을 먼저 검사할까?", language: Korean },
            Turn { text: "인덱스 쪽", language: Korean },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50_06",
        category: "topic_restoration",
        turns: &[
            Turn { text: "Inspect the cache.", language: English },
            Turn { text: "Inspect the queue.", language: English },
            Turn { text: "Go back to the cache and repair it.", language: English },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50_07",
        category: "definition_grounding",
        turns: &[Turn { text: "무루 means inspect.", language: English }],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50_08",
        category: "language_report_not_result",
        turns: &[
            Turn { text: "Inspect the cache.", language: English },
            Turn { text: "I completed it.", language: English },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50_09",
        category: "verified_result_evidence_absent",
        turns: &[
            Turn { text: "로그를 검사해", language: Korean },
            Turn { text: "검증된 결과가 뭐야?", language: Korean },
        ],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50_10",
        category: "korean_compound_condition",
        turns: &[Turn { text: "캐시를 검사하고 오래됐거나 손상됐으면 수리해", language: Korean }],
        expected_disposition: Grounded,
    },
    Case {
        id: "R50_11",
        category: "mixed_target_guard_fails_closed",
        turns: &[
            Turn { text: "Inspect the cache and if the cache is stale or the queue is empty, repair the cache.", language: English },
            Turn { text: "Use the same procedure for the index.", language: English },
        ],
        expected_disposition: ClarificationRequired,
    },
    Case {
        id: "R50_12",
        category: "backchannel_state_binding",
        turns: &[
            Turn { text: "파일을 검사해", language: Korean },
            Turn { text: "응", language: Korean },
        ],
        expected_disposition: semantic_core_adapters::ConversationTurnDispositionIR::BackchannelOnly,
    },
];

fn main() {
    emit("R50_LANGUAGE_CORTEX_INTEGRATION_DIAGNOSTIC", CASES);
}
