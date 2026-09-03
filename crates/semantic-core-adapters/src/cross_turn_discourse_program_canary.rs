//! Frozen R46 diagnostic suite for cross-turn typed discourse programs.

mod discourse_program_canary_support;

use discourse_program_canary_support::{emit, Case, Turn};
use dockable_semantic_core::PlanIntentIR::{Execute, Investigate, Repair};
use semantic_core_adapters::{
    ConversationTurnDispositionIR::{ClarificationRequired, Grounded},
    LanguageCodeIR::{English, Korean},
};

const CASES: &[Case] = &[
    Case {
        id: "R46_01",
        category: "ko_pair",
        turns: &[
            Turn {
                text: "캐시를 확인하고 수리해",
                language: Korean,
            },
            Turn {
                text: "인덱스도 똑같이 해",
                language: Korean,
            },
        ],
        expected: &[(Investigate, "인덱스"), (Repair, "인덱스")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46_02",
        category: "ko_chain",
        turns: &[
            Turn {
                text: "파일을 읽고 변환하고 저장해",
                language: Korean,
            },
            Turn {
                text: "문서도 같은 방식으로 해",
                language: Korean,
            },
        ],
        expected: &[(Execute, "문서"), (Execute, "문서"), (Execute, "문서")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46_03",
        category: "en_pair",
        turns: &[
            Turn {
                text: "Inspect and repair the cache.",
                language: English,
            },
            Turn {
                text: "Do the same for the index.",
                language: English,
            },
        ],
        expected: &[(Investigate, "index"), (Repair, "index")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46_04",
        category: "en_chain",
        turns: &[
            Turn {
                text: "Read, transform, and save the file.",
                language: English,
            },
            Turn {
                text: "Apply the same to the document.",
                language: English,
            },
        ],
        expected: &[
            (Execute, "document"),
            (Execute, "document"),
            (Execute, "document"),
        ],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46_05",
        category: "ko_to_en",
        turns: &[
            Turn {
                text: "캐시를 검사하고 고쳐",
                language: Korean,
            },
            Turn {
                text: "Do the same for the queue.",
                language: English,
            },
        ],
        expected: &[(Investigate, "queue"), (Repair, "queue")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46_06",
        category: "en_to_ko",
        turns: &[
            Turn {
                text: "Inspect and repair the server.",
                language: English,
            },
            Turn {
                text: "워커도 똑같이 해",
                language: Korean,
            },
        ],
        expected: &[(Investigate, "워커"), (Repair, "워커")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46_07",
        category: "ko_novel_target",
        turns: &[
            Turn {
                text: "백업을 확인하고 복구해",
                language: Korean,
            },
            Turn {
                text: "샤드도 똑같이 해",
                language: Korean,
            },
        ],
        expected: &[(Investigate, "샤드"), (Repair, "샤드")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46_08",
        category: "en_novel_target",
        turns: &[
            Turn {
                text: "Open and restore the repository.",
                language: English,
            },
            Turn {
                text: "Do the same operation for the archive.",
                language: English,
            },
        ],
        expected: &[(Execute, "archive"), (Repair, "archive")],
        expected_disposition: Grounded,
        expect_program_instantiation: true,
        expect_program_count: 2,
        expect_elliptical_ambiguity: false,
    },
    Case {
        id: "R46_09",
        category: "bare_repeat_fails_closed",
        turns: &[
            Turn {
                text: "파일을 읽고 저장해",
                language: Korean,
            },
            Turn {
                text: "그대로 해",
                language: Korean,
            },
        ],
        expected: &[],
        expected_disposition: ClarificationRequired,
        expect_program_instantiation: false,
        expect_program_count: 1,
        expect_elliptical_ambiguity: true,
    },
    Case {
        id: "R46_10",
        category: "negated_program_fails_closed",
        turns: &[
            Turn {
                text: "캐시를 확인하고 삭제하지 마",
                language: Korean,
            },
            Turn {
                text: "인덱스도 똑같이 해",
                language: Korean,
            },
        ],
        expected: &[],
        expected_disposition: ClarificationRequired,
        expect_program_instantiation: false,
        expect_program_count: 1,
        expect_elliptical_ambiguity: true,
    },
    Case {
        id: "R46_11",
        category: "quoted_program_fails_closed",
        turns: &[
            Turn {
                text: "철수가 ‘캐시를 확인하고 수리해’라고 말했다",
                language: Korean,
            },
            Turn {
                text: "인덱스도 똑같이 해",
                language: Korean,
            },
        ],
        expected: &[],
        expected_disposition: ClarificationRequired,
        expect_program_instantiation: false,
        expect_program_count: 0,
        expect_elliptical_ambiguity: true,
    },
    Case {
        id: "R46_12",
        category: "missing_program_fails_closed",
        turns: &[Turn {
            text: "Do the same for the index.",
            language: English,
        }],
        expected: &[],
        expected_disposition: ClarificationRequired,
        expect_program_instantiation: false,
        expect_program_count: 0,
        expect_elliptical_ambiguity: true,
    },
];

fn main() {
    emit("R46_DIAGNOSTIC", CASES);
}
