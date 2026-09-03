//! Frozen R49 diagnostic suite for typed compound guarded-condition expressions.

mod compound_guard_expression_canary_support;

use compound_guard_expression_canary_support::{emit, Case, Turn};
use semantic_core_adapters::LanguageCodeIR::{English, Korean};

const CASES: &[Case] = &[
    Case {
        id: "R49_01",
        category: "english_or",
        turns: &[Turn { text: "Inspect the cache and if the cache is stale or damaged, repair the cache.", language: English }],
        expected_signature: Some("ANY(ATOM:STALE,ATOM:DAMAGED)"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49_02",
        category: "english_and",
        turns: &[Turn { text: "Inspect the cache and if the cache has an error and is stale, repair the cache.", language: English }],
        expected_signature: Some("ALL(ATOM:ERROR_PRESENT,ATOM:STALE)"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49_03",
        category: "english_precedence",
        turns: &[Turn { text: "Inspect the cache and if the cache is stale or damaged and invalid, repair the cache.", language: English }],
        expected_signature: Some("ANY(ATOM:STALE,ALL(ATOM:DAMAGED,ATOM:INVALID))"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49_04",
        category: "english_parenthesized_scope",
        turns: &[Turn { text: "Inspect the cache and if the cache is stale and (damaged or invalid), repair the cache.", language: English }],
        expected_signature: Some("ALL(ATOM:STALE,ANY(ATOM:DAMAGED,ATOM:INVALID))"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49_05",
        category: "korean_or",
        turns: &[Turn { text: "캐시를 검사하고 캐시가 오래됐거나 손상됐으면 수리해", language: Korean }],
        expected_signature: Some("ANY(ATOM:STALE,ATOM:DAMAGED)"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49_06",
        category: "korean_and",
        turns: &[Turn { text: "캐시를 검사하고 캐시에 오류가 있고 오래됐으면 수리해", language: Korean }],
        expected_signature: Some("ALL(ATOM:ERROR_PRESENT,ATOM:STALE)"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49_07",
        category: "korean_precedence",
        turns: &[Turn { text: "캐시를 검사하고 캐시가 오래됐거나 손상됐고 유효하지 않으면 수리해", language: Korean }],
        expected_signature: Some("ANY(ATOM:STALE,ALL(ATOM:DAMAGED,NOT(ATOM:VALID)))"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49_08",
        category: "english_local_negation",
        turns: &[Turn { text: "Inspect the cache and if the cache is not stale or the cache is damaged, repair the cache.", language: English }],
        expected_signature: Some("ANY(NOT(ATOM:STALE),ATOM:DAMAGED)"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49_09",
        category: "korean_local_negation",
        turns: &[Turn { text: "캐시를 검사하고 캐시가 오래되지 않았거나 손상됐으면 수리해", language: Korean }],
        expected_signature: Some("ANY(NOT(ATOM:STALE),ATOM:DAMAGED)"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49_10",
        category: "cross_language_rebinding",
        turns: &[
            Turn { text: "Inspect the cache and if the cache is stale or damaged, repair the cache.", language: English },
            Turn { text: "인덱스도 같은 절차로 해", language: Korean },
        ],
        expected_signature: Some("ANY(ATOM:STALE,ATOM:DAMAGED)"),
        expected_guarded_programs: 2,
        expected_pending_commitments: 2,
        expected_clarification: false,
        require_stable_rebinding: true,
    },
    Case {
        id: "R49_11",
        category: "language_claim_has_no_guard_authority",
        turns: &[
            Turn { text: "Inspect the cache and if the cache is stale or damaged, repair the cache.", language: English },
            Turn { text: "Use the same procedure for the queue.", language: English },
            Turn { text: "The queue is stale or damaged.", language: English },
        ],
        expected_signature: Some("ANY(ATOM:STALE,ATOM:DAMAGED)"),
        expected_guarded_programs: 2,
        expected_pending_commitments: 2,
        expected_clarification: false,
        require_stable_rebinding: true,
    },
    Case {
        id: "R49_12",
        category: "mixed_target_compound_fails_closed",
        turns: &[
            Turn { text: "Inspect the cache and if the cache is stale or the queue is empty, repair the cache.", language: English },
            Turn { text: "Use the same procedure for the index.", language: English },
        ],
        expected_signature: None,
        expected_guarded_programs: 0,
        expected_pending_commitments: 1,
        expected_clarification: true,
        require_stable_rebinding: false,
    },
];

fn main() {
    emit("R49_COMPOUND_GUARD_EXPRESSION_DIAGNOSTIC", CASES);
}
