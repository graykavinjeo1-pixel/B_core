//! Frozen R49 held-out transfer suite. Execute only after diagnostic success.

mod compound_guard_expression_canary_support;

use compound_guard_expression_canary_support::{emit, Case, Turn};
use semantic_core_adapters::LanguageCodeIR::{English, Korean};

const CASES: &[Case] = &[
    Case {
        id: "R49T_01",
        category: "fresh_english_synonym_or",
        turns: &[Turn { text: "Inspect the service and if the service is outdated or corrupt, restore the service.", language: English }],
        expected_signature: Some("ANY(ATOM:STALE,ATOM:DAMAGED)"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49T_02",
        category: "fresh_korean_predicate_or",
        turns: &[Turn { text: "로그를 확인하고 로그에 오류가 있거나 비었으면 복구해", language: Korean }],
        expected_signature: Some("ANY(ATOM:ERROR_PRESENT,ATOM:EMPTY)"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49T_03",
        category: "fresh_english_nested_scope",
        turns: &[Turn { text: "Inspect the archive and if the archive is invalid or (empty and damaged), repair the archive.", language: English }],
        expected_signature: Some("ANY(ATOM:INVALID,ALL(ATOM:EMPTY,ATOM:DAMAGED))"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49T_04",
        category: "fresh_nested_cross_language_rebinding",
        turns: &[
            Turn { text: "Inspect the cache and if the cache is stale and (damaged or invalid), repair the cache.", language: English },
            Turn { text: "미러도 같은 절차로 해", language: Korean },
        ],
        expected_signature: Some("ALL(ATOM:STALE,ANY(ATOM:DAMAGED,ATOM:INVALID))"),
        expected_guarded_programs: 2,
        expected_pending_commitments: 2,
        expected_clarification: false,
        require_stable_rebinding: true,
    },
    Case {
        id: "R49T_05",
        category: "fresh_korean_negated_and",
        turns: &[Turn { text: "메타데이터를 검사하고 메타데이터가 유효하지 않고 오래됐으면 수리해", language: Korean }],
        expected_signature: Some("ALL(NOT(ATOM:VALID),ATOM:STALE)"),
        expected_guarded_programs: 1,
        expected_pending_commitments: 1,
        expected_clarification: false,
        require_stable_rebinding: false,
    },
    Case {
        id: "R49T_06",
        category: "fresh_korean_to_english_rebinding",
        turns: &[
            Turn { text: "백업을 확인하고 백업에 오류가 있거나 손상됐으면 복구해", language: Korean },
            Turn { text: "Apply the same workflow to the archive.", language: English },
        ],
        expected_signature: Some("ANY(ATOM:ERROR_PRESENT,ATOM:DAMAGED)"),
        expected_guarded_programs: 2,
        expected_pending_commitments: 2,
        expected_clarification: false,
        require_stable_rebinding: true,
    },
    Case {
        id: "R49T_07",
        category: "fresh_reported_claim_non_authority",
        turns: &[
            Turn { text: "Inspect the worker and if the worker is stale and damaged, repair the worker.", language: English },
            Turn { text: "Use the same procedure for the service.", language: English },
            Turn { text: "Alice says the service is stale and damaged.", language: English },
        ],
        expected_signature: Some("ALL(ATOM:STALE,ATOM:DAMAGED)"),
        expected_guarded_programs: 2,
        expected_pending_commitments: 2,
        expected_clarification: false,
        require_stable_rebinding: true,
    },
    Case {
        id: "R49T_08",
        category: "fresh_korean_mixed_target_fails_closed",
        turns: &[
            Turn { text: "캐시를 검사하고 캐시가 오래됐거나 큐가 비었으면 캐시를 수리해", language: Korean },
            Turn { text: "인덱스도 같은 절차로 해", language: Korean },
        ],
        expected_signature: None,
        expected_guarded_programs: 0,
        expected_pending_commitments: 1,
        expected_clarification: true,
        require_stable_rebinding: false,
    },
];

fn main() {
    emit("R49_COMPOUND_GUARD_EXPRESSION_HELD_OUT_TRANSFER", CASES);
}
