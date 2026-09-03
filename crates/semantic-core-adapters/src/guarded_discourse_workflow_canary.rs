//! Frozen R47 diagnostic suite for guarded cross-turn discourse workflows.

mod guarded_workflow_canary_support;

use dockable_semantic_core::PlanIntentIR::{Investigate, Repair};
use guarded_workflow_canary_support::{emit, Case, Turn};
use semantic_core_adapters::{
    ConversationTurnDispositionIR::{ClarificationRequired, Grounded},
    LanguageCodeIR::{English, Korean},
};

const CASES: &[Case] = &[
    Case {
        id: "R47_01",
        category: "ko_guarded_rebind",
        turns: &[
            Turn { text: "캐시를 검사하고 캐시에 문제가 있으면 수리해", language: Korean },
            Turn { text: "인덱스도 같은 절차로 해", language: Korean },
        ],
        expected_active: &[(Investigate, "인덱스")],
        expected_pending: &[(Repair, "인덱스")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47_02",
        category: "en_guarded_rebind",
        turns: &[
            Turn { text: "Inspect the cache and if the cache is stale, repair the cache.", language: English },
            Turn { text: "Use the same procedure for the queue.", language: English },
        ],
        expected_active: &[(Investigate, "queue")],
        expected_pending: &[(Repair, "queue")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47_03",
        category: "ko_to_en_guarded_rebind",
        turns: &[
            Turn { text: "백업을 확인하고 백업이 손상됐으면 복구해", language: Korean },
            Turn { text: "Apply the same workflow to the archive.", language: English },
        ],
        expected_active: &[(Investigate, "archive")],
        expected_pending: &[(Repair, "archive")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47_04",
        category: "en_to_ko_guarded_rebind",
        turns: &[
            Turn { text: "Inspect the server and if the server is unhealthy, repair the server.", language: English },
            Turn { text: "워커도 같은 방식으로 해", language: Korean },
        ],
        expected_active: &[(Investigate, "워커")],
        expected_pending: &[(Repair, "워커")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47_05",
        category: "ko_negative_antecedent_preserved",
        turns: &[
            Turn { text: "캐시를 검사하고 캐시가 유효하지 않으면 수리해", language: Korean },
            Turn { text: "인덱스도 똑같이 해", language: Korean },
        ],
        expected_active: &[(Investigate, "인덱스")],
        expected_pending: &[(Repair, "인덱스")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47_06",
        category: "en_negative_antecedent_preserved",
        turns: &[
            Turn { text: "Inspect the cache and if the cache is not valid, repair the cache.", language: English },
            Turn { text: "Do the same for the index.", language: English },
        ],
        expected_active: &[(Investigate, "index")],
        expected_pending: &[(Repair, "index")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47_07",
        category: "ko_open_compound_target",
        turns: &[
            Turn { text: "설정 파일을 검사하고 설정 파일이 오래됐으면 수리해", language: Korean },
            Turn { text: "메타데이터 저장소도 같은 절차로 해", language: Korean },
        ],
        expected_active: &[(Investigate, "메타데이터 저장소")],
        expected_pending: &[(Repair, "메타데이터 저장소")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47_08",
        category: "en_open_compound_target",
        turns: &[
            Turn { text: "Inspect the build manifest and if the build manifest is stale, repair the build manifest.", language: English },
            Turn { text: "Repeat that workflow for the release index.", language: English },
        ],
        expected_active: &[(Investigate, "release index")],
        expected_pending: &[(Repair, "release index")],
        expected_disposition: Grounded,
        expect_guarded_instantiation: true,
        expect_program_count: 2,
        expect_guarded_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R47_09",
        category: "bare_repeat_fails_closed",
        turns: &[
            Turn { text: "캐시를 검사하고 캐시가 오래됐으면 수리해", language: Korean },
            Turn { text: "같은 절차로 해", language: Korean },
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
        id: "R47_10",
        category: "quoted_workflow_fails_closed",
        turns: &[
            Turn { text: "철수가 ‘캐시를 검사하고 문제가 있으면 수리해’라고 말했다", language: Korean },
            Turn { text: "인덱스도 같은 절차로 해", language: Korean },
        ],
        expected_active: &[],
        expected_pending: &[],
        expected_disposition: ClarificationRequired,
        expect_guarded_instantiation: false,
        expect_program_count: 0,
        expect_guarded_program_count: 0,
        expect_elliptical_ambiguity: true,
    },
    Case {
        id: "R47_11",
        category: "counterfactual_workflow_fails_closed",
        turns: &[
            Turn { text: "캐시를 검사했고 문제가 있었더라면 수리했을 텐데", language: Korean },
            Turn { text: "인덱스도 같은 절차로 해", language: Korean },
        ],
        expected_active: &[],
        expected_pending: &[],
        expected_disposition: ClarificationRequired,
        expect_guarded_instantiation: false,
        expect_program_count: 0,
        expect_guarded_program_count: 0,
        expect_elliptical_ambiguity: true,
    },
    Case {
        id: "R47_12",
        category: "mixed_target_guard_fails_closed",
        turns: &[
            Turn { text: "캐시를 검사하고 큐가 오래됐으면 큐를 수리해", language: Korean },
            Turn { text: "인덱스도 같은 절차로 해", language: Korean },
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
    emit("R47_GUARDED_WORKFLOW_DIAGNOSTIC", CASES);
}
