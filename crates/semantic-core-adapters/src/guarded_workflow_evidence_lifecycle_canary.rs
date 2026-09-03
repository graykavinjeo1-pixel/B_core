//! Frozen R48 diagnostic suite for hash-bound guarded-program evidence lifecycles.

mod guarded_evidence_lifecycle_canary_support;

use guarded_evidence_lifecycle_canary_support::{emit, Case, EvidencePlan, Turn};
use semantic_core_adapters::{
    DeferredCommitmentStatusIR::{Activated, ConditionPending, Contradicted},
    LanguageCodeIR::{English, Korean},
};

const CASES: &[Case] = &[
    Case {
        id: "R48_01",
        category: "ko_verified_rebound_activation",
        turns: &[
            Turn {
                text: "캐시를 검사하고 캐시에 문제가 있으면 수리해",
                language: Korean,
            },
            Turn {
                text: "인덱스도 같은 절차로 해",
                language: Korean,
            },
        ],
        evidence_plan: EvidencePlan::SatisfyRebound,
        expected_source_status: ConditionPending,
        expected_rebound_status: Activated,
        expected_activated_subjects: &["인덱스"],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48_02",
        category: "en_verified_rebound_activation",
        turns: &[
            Turn {
                text: "Inspect the cache and if the cache is stale, repair the cache.",
                language: English,
            },
            Turn {
                text: "Use the same procedure for the queue.",
                language: English,
            },
        ],
        evidence_plan: EvidencePlan::SatisfyRebound,
        expected_source_status: ConditionPending,
        expected_rebound_status: Activated,
        expected_activated_subjects: &["queue"],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48_03",
        category: "ko_negated_guard_activation",
        turns: &[
            Turn {
                text: "캐시를 검사하고 캐시가 유효하지 않으면 수리해",
                language: Korean,
            },
            Turn {
                text: "인덱스도 똑같이 해",
                language: Korean,
            },
        ],
        evidence_plan: EvidencePlan::SatisfyRebound,
        expected_source_status: ConditionPending,
        expected_rebound_status: Activated,
        expected_activated_subjects: &["인덱스"],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48_04",
        category: "cross_language_verified_activation",
        turns: &[
            Turn {
                text: "백업을 확인하고 백업이 손상됐으면 복구해",
                language: Korean,
            },
            Turn {
                text: "Apply the same workflow to the archive.",
                language: English,
            },
        ],
        evidence_plan: EvidencePlan::SatisfyRebound,
        expected_source_status: ConditionPending,
        expected_rebound_status: Activated,
        expected_activated_subjects: &["archive"],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48_05",
        category: "verified_contradiction_retires_rebound",
        turns: &[
            Turn {
                text: "Inspect the cache and if the cache is stale, repair the cache.",
                language: English,
            },
            Turn {
                text: "Do the same for the index.",
                language: English,
            },
        ],
        evidence_plan: EvidencePlan::ContradictRebound,
        expected_source_status: ConditionPending,
        expected_rebound_status: Contradicted,
        expected_activated_subjects: &[],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48_06",
        category: "language_claim_cannot_activate",
        turns: &[
            Turn {
                text: "캐시를 검사하고 캐시에 문제가 있으면 수리해",
                language: Korean,
            },
            Turn {
                text: "인덱스도 같은 절차로 해",
                language: Korean,
            },
            Turn {
                text: "인덱스에 문제가 있어",
                language: Korean,
            },
        ],
        evidence_plan: EvidencePlan::None,
        expected_source_status: ConditionPending,
        expected_rebound_status: ConditionPending,
        expected_activated_subjects: &[],
        expected_receipts: 0,
        expected_rejections: 0,
    },
    Case {
        id: "R48_07",
        category: "reported_claim_cannot_activate",
        turns: &[
            Turn {
                text: "Inspect the cache and if the cache is stale, repair the cache.",
                language: English,
            },
            Turn {
                text: "Use the same procedure for the queue.",
                language: English,
            },
            Turn {
                text: "Alice says the queue is stale.",
                language: English,
            },
        ],
        evidence_plan: EvidencePlan::None,
        expected_source_status: ConditionPending,
        expected_rebound_status: ConditionPending,
        expected_activated_subjects: &[],
        expected_receipts: 0,
        expected_rejections: 0,
    },
    Case {
        id: "R48_08",
        category: "wrong_condition_hash_rejected",
        turns: &[
            Turn {
                text: "캐시를 검사하고 캐시가 오래됐으면 수리해",
                language: Korean,
            },
            Turn {
                text: "인덱스도 같은 절차로 해",
                language: Korean,
            },
        ],
        evidence_plan: EvidencePlan::WrongConditionHash,
        expected_source_status: ConditionPending,
        expected_rebound_status: ConditionPending,
        expected_activated_subjects: &[],
        expected_receipts: 0,
        expected_rejections: 1,
    },
    Case {
        id: "R48_09",
        category: "foreign_commitment_id_rejected",
        turns: &[
            Turn {
                text: "Inspect the cache and if the cache is stale, repair the cache.",
                language: English,
            },
            Turn {
                text: "Do the same for the index.",
                language: English,
            },
        ],
        evidence_plan: EvidencePlan::WrongCommitmentId,
        expected_source_status: ConditionPending,
        expected_rebound_status: ConditionPending,
        expected_activated_subjects: &[],
        expected_receipts: 0,
        expected_rejections: 1,
    },
    Case {
        id: "R48_10",
        category: "verified_evidence_is_exactly_once",
        turns: &[
            Turn {
                text: "캐시를 검사하고 캐시에 문제가 있으면 수리해",
                language: Korean,
            },
            Turn {
                text: "인덱스도 같은 절차로 해",
                language: Korean,
            },
        ],
        evidence_plan: EvidencePlan::ReplaySatisfied,
        expected_source_status: ConditionPending,
        expected_rebound_status: Activated,
        expected_activated_subjects: &["인덱스"],
        expected_receipts: 1,
        expected_rejections: 1,
    },
    Case {
        id: "R48_11",
        category: "source_evidence_does_not_activate_rebound",
        turns: &[
            Turn {
                text: "Inspect the cache and if the cache is stale, repair the cache.",
                language: English,
            },
            Turn {
                text: "Use the same procedure for the queue.",
                language: English,
            },
        ],
        evidence_plan: EvidencePlan::SatisfySource,
        expected_source_status: Activated,
        expected_rebound_status: ConditionPending,
        expected_activated_subjects: &["cache"],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48_12",
        category: "pending_program_commitment_cross_link_integrity",
        turns: &[
            Turn {
                text: "로그를 분석하고 오류가 있으면 고쳐",
                language: Korean,
            },
            Turn {
                text: "메트릭도 같은 절차로 해",
                language: Korean,
            },
        ],
        evidence_plan: EvidencePlan::None,
        expected_source_status: ConditionPending,
        expected_rebound_status: ConditionPending,
        expected_activated_subjects: &[],
        expected_receipts: 0,
        expected_rejections: 0,
    },
];

fn main() {
    emit("R48_GUARDED_EVIDENCE_LIFECYCLE_DIAGNOSTIC", CASES);
}
