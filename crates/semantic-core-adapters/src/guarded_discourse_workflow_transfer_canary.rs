//! Frozen R47 held-out transfer suite. Execute only after diagnostic success.

mod guarded_workflow_canary_support;

use dockable_semantic_core::PlanIntentIR::{Investigate, Repair};
use guarded_workflow_canary_support::{emit, Case, Turn};
use semantic_core_adapters::{
    ConversationTurnDispositionIR::{ClarificationRequired, Grounded},
    LanguageCodeIR::{English, Korean},
};

const CASES: &[Case] = &[
    Case {
        id: "R47T_01",
        category: "fresh_ko_zero_argument_guard",
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
        expected_active: &[(Investigate, "메트릭")],
        expected_pending: &[(Repair, "메트릭")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47T_02",
        category: "fresh_en_pronominal_guard",
        turns: &[
            Turn {
                text: "Inspect the service and if it is stale, repair it.",
                language: English,
            },
            Turn {
                text: "Apply the same procedure to the worker.",
                language: English,
            },
        ],
        expected_active: &[(Investigate, "worker")],
        expected_pending: &[(Repair, "worker")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47T_03",
        category: "fresh_ko_to_en",
        turns: &[
            Turn {
                text: "저장소를 확인하고 저장소가 깨졌으면 복구해",
                language: Korean,
            },
            Turn {
                text: "Repeat the workflow for the mirror.",
                language: English,
            },
        ],
        expected_active: &[(Investigate, "mirror")],
        expected_pending: &[(Repair, "mirror")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47T_04",
        category: "fresh_en_to_ko",
        turns: &[
            Turn {
                text: "Inspect the archive and if the archive is damaged, repair the archive.",
                language: English,
            },
            Turn {
                text: "백업도 똑같이 해",
                language: Korean,
            },
        ],
        expected_active: &[(Investigate, "백업")],
        expected_pending: &[(Repair, "백업")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47T_05",
        category: "topic_selects_prior_guarded_workflow",
        turns: &[
            Turn {
                text: "캐시를 검사하고 캐시가 오래됐으면 수리해",
                language: Korean,
            },
            Turn {
                text: "큐를 검사하고 큐가 비었으면 수리해",
                language: Korean,
            },
            Turn {
                text: "캐시 이야기로 돌아가자",
                language: Korean,
            },
            Turn {
                text: "인덱스도 같은 절차로 해",
                language: Korean,
            },
        ],
        expected_active: &[(Investigate, "인덱스")],
        expected_pending: &[(Repair, "인덱스")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 3,
        expect_guarded_program_count: 3,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47T_06",
        category: "missing_workflow_fails_closed",
        turns: &[Turn {
            text: "Use the same guarded procedure for the index.",
            language: English,
        }],
        expected_active: &[],
        expected_pending: &[],
        expected_disposition: ClarificationRequired,
        expect_guarded_instantiation: false,
        expect_program_count: 0,
        expect_guarded_program_count: 0,
        expect_elliptical_ambiguity: true,
    },
    Case {
        id: "R47T_07",
        category: "quoted_ellipsis_fails_closed",
        turns: &[
            Turn {
                text: "Inspect the cache and if the cache is stale, repair the cache.",
                language: English,
            },
            Turn {
                text: "Alice said, ‘do the same for the index.’",
                language: English,
            },
        ],
        expected_active: &[],
        expected_pending: &[],
        expected_disposition: ClarificationRequired,
        expect_guarded_instantiation: false,
        expect_program_count: 1,
        expect_guarded_program_count: 1,
        expect_elliptical_ambiguity: true,
    },
    Case {
        id: "R47T_08",
        category: "foreign_guard_target_fails_closed",
        turns: &[
            Turn {
                text: "Inspect the cache and if the queue is stale, repair the queue.",
                language: English,
            },
            Turn {
                text: "Do the same for the index.",
                language: English,
            },
        ],
        expected_active: &[],
        expected_pending: &[],
        expected_disposition: ClarificationRequired,
        expect_guarded_instantiation: false,
        expect_program_count: 1,
        expect_guarded_program_count: 0,
        expect_elliptical_ambiguity: true,
    },
];

fn main() {
    emit("R47_GUARDED_WORKFLOW_HELD_OUT_TRANSFER", CASES);
}
