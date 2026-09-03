//! Frozen R48 held-out transfer suite. Execute only after diagnostic success.

mod guarded_evidence_lifecycle_canary_support;

use guarded_evidence_lifecycle_canary_support::{emit, Case, EvidencePlan, Turn};
use semantic_core_adapters::{
    DeferredCommitmentStatusIR::{Activated, ConditionPending, Contradicted},
    LanguageCodeIR::{English, Korean},
};

const CASES: &[Case] = &[
    Case {
        id: "R48T_01",
        category: "fresh_ko_zero_argument_activation",
        turns: &[
            Turn { text: "로그를 분석하고 오류가 있으면 고쳐", language: Korean },
            Turn { text: "메트릭도 같은 절차로 해", language: Korean },
        ],
        evidence_plan: EvidencePlan::SatisfyRebound,
        expected_source_status: ConditionPending,
        expected_rebound_status: Activated,
        expected_activated_subjects: &["메트릭"],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48T_02",
        category: "fresh_en_pronominal_activation",
        turns: &[
            Turn { text: "Inspect the service and if it is stale, repair it.", language: English },
            Turn { text: "Apply the same procedure to the worker.", language: English },
        ],
        evidence_plan: EvidencePlan::SatisfyRebound,
        expected_source_status: ConditionPending,
        expected_rebound_status: Activated,
        expected_activated_subjects: &["worker"],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48T_03",
        category: "fresh_ko_to_en_activation",
        turns: &[
            Turn { text: "저장소를 확인하고 저장소가 깨졌으면 복구해", language: Korean },
            Turn { text: "Repeat the workflow for the mirror.", language: English },
        ],
        evidence_plan: EvidencePlan::SatisfyRebound,
        expected_source_status: ConditionPending,
        expected_rebound_status: Activated,
        expected_activated_subjects: &["mirror"],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48T_04",
        category: "fresh_en_to_ko_contradiction",
        turns: &[
            Turn { text: "Inspect the archive and if the archive is damaged, repair the archive.", language: English },
            Turn { text: "백업도 똑같이 해", language: Korean },
        ],
        evidence_plan: EvidencePlan::ContradictRebound,
        expected_source_status: ConditionPending,
        expected_rebound_status: Contradicted,
        expected_activated_subjects: &[],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48T_05",
        category: "fresh_open_compound_activation",
        turns: &[
            Turn { text: "설정 파일을 검사하고 설정 파일이 오래됐으면 수리해", language: Korean },
            Turn { text: "메타데이터 저장소도 같은 절차로 해", language: Korean },
        ],
        evidence_plan: EvidencePlan::SatisfyRebound,
        expected_source_status: ConditionPending,
        expected_rebound_status: Activated,
        expected_activated_subjects: &["메타데이터 저장소"],
        expected_receipts: 1,
        expected_rejections: 0,
    },
    Case {
        id: "R48T_06",
        category: "fresh_compound_wrong_hash_rejected",
        turns: &[
            Turn { text: "Inspect the build manifest and if the build manifest is stale, repair the build manifest.", language: English },
            Turn { text: "Repeat that workflow for the release index.", language: English },
        ],
        evidence_plan: EvidencePlan::WrongConditionHash,
        expected_source_status: ConditionPending,
        expected_rebound_status: ConditionPending,
        expected_activated_subjects: &[],
        expected_receipts: 0,
        expected_rejections: 1,
    },
    Case {
        id: "R48T_07",
        category: "fresh_cross_language_report_non_authority",
        turns: &[
            Turn { text: "캐시를 검사하고 캐시가 오래됐으면 수리해", language: Korean },
            Turn { text: "Apply the same procedure to the queue.", language: English },
            Turn { text: "Bob says the queue is stale.", language: English },
        ],
        evidence_plan: EvidencePlan::None,
        expected_source_status: ConditionPending,
        expected_rebound_status: ConditionPending,
        expected_activated_subjects: &[],
        expected_receipts: 0,
        expected_rejections: 0,
    },
    Case {
        id: "R48T_08",
        category: "fresh_source_rebound_isolation",
        turns: &[
            Turn { text: "캐시를 검사하고 캐시가 유효하지 않으면 수리해", language: Korean },
            Turn { text: "인덱스도 똑같이 해", language: Korean },
        ],
        evidence_plan: EvidencePlan::SatisfySource,
        expected_source_status: Activated,
        expected_rebound_status: ConditionPending,
        expected_activated_subjects: &["캐시"],
        expected_receipts: 1,
        expected_rejections: 0,
    },
];

fn main() {
    emit("R48_GUARDED_EVIDENCE_LIFECYCLE_HELD_OUT_TRANSFER", CASES);
}
