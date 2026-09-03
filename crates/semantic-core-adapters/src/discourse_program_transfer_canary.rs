//! Frozen R46 held-out transfer suite. Execute only after the diagnostic passes.

mod discourse_program_canary_support;

use discourse_program_canary_support::{emit, Case, Turn};
use dockable_semantic_core::PlanIntentIR::{Create, Execute, Investigate, Repair};
use semantic_core_adapters::{
    ConversationTurnDispositionIR::{ClarificationRequired, Grounded},
    LanguageCodeIR::{English, Korean},
};

const CASES: &[Case] = &[
    Case {
        id: "R46T_01",
        category: "fresh_ko_pair",
        turns: &[
            Turn {
                text: "로그를 분석하고 요약해",
                language: Korean,
            },
            Turn {
                text: "메트릭도 똑같이 해줘",
                language: Korean,
            },
        ],
        expected: &[(Investigate, "메트릭"), (Investigate, "메트릭")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46T_02",
        category: "fresh_en_pair",
        turns: &[
            Turn {
                text: "Analyze and document the project.",
                language: English,
            },
            Turn {
                text: "Use the same procedure for the repository.",
                language: English,
            },
        ],
        expected: &[(Investigate, "repository"), (Create, "repository")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46T_03",
        category: "fresh_ko_chain",
        turns: &[
            Turn {
                text: "소스 코드를 읽고 고치고 검증해",
                language: Korean,
            },
            Turn {
                text: "설정 파일도 같은 방식으로 해",
                language: Korean,
            },
        ],
        expected: &[
            (Execute, "설정 파일"),
            (Repair, "설정 파일"),
            (Investigate, "설정 파일"),
        ],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46T_04",
        category: "fresh_en_chain",
        turns: &[
            Turn {
                text: "Read, convert, and save the report.",
                language: English,
            },
            Turn {
                text: "Repeat that workflow for the manifest.",
                language: English,
            },
        ],
        expected: &[
            (Execute, "manifest"),
            (Execute, "manifest"),
            (Execute, "manifest"),
        ],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46T_05",
        category: "fresh_cross_language",
        turns: &[
            Turn {
                text: "Inspect and repair the backup.",
                language: English,
            },
            Turn {
                text: "아카이브도 같은 방식으로 해",
                language: Korean,
            },
        ],
        expected: &[(Investigate, "아카이브"), (Repair, "아카이브")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46T_06",
        category: "fresh_cross_language",
        turns: &[
            Turn {
                text: "큐를 확인하고 복구해",
                language: Korean,
            },
            Turn {
                text: "Apply the same workflow to the worker.",
                language: English,
            },
        ],
        expected: &[(Investigate, "worker"), (Repair, "worker")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46T_07",
        category: "bare_repeat_stays_ambiguous",
        turns: &[
            Turn {
                text: "Open and save the file.",
                language: English,
            },
            Turn {
                text: "Do the same.",
                language: English,
            },
        ],
        expected: &[],
        expected_disposition: ClarificationRequired,
        expect_program_instantiation: false,
        expect_program_count: 1,
        expect_elliptical_ambiguity: true,
    },
    Case {
        id: "R46T_08",
        category: "negative_stays_blocked",
        turns: &[
            Turn {
                text: "Inspect and do not delete the backup.",
                language: English,
            },
            Turn {
                text: "Apply the same to the archive.",
                language: English,
            },
        ],
        expected: &[],
        expected_disposition: ClarificationRequired,
        expect_program_instantiation: false,
        expect_program_count: 1,
        expect_elliptical_ambiguity: true,
    },
];

fn main() {
    emit("R46_HELD_OUT_TRANSFER", CASES);
}
