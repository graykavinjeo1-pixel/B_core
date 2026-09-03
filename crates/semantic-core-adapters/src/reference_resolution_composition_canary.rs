//! Frozen R58 pre-repair diagnostic for compositional reference resolution.

mod reference_resolution_composition_canary_support;

use reference_resolution_composition_canary_support::{emit, Case, Expectation, Turn};
use semantic_core_adapters::LanguageCodeIR::{English, Korean};

const CASES: &[Case<'_>] = &[
    Case {
        id: "R58_01",
        category: "english_possessive_plus_demonstrative",
        setup: &[Turn {
            text: "Inspect the cache and repair the queue.",
            language: English,
        }],
        follow: Turn {
            text: "Inspect its status, then analyze that object.",
            language: English,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("queue", 2)],
            forbidden_markers: &["its", "that object"],
            selected_antecedents: &[("queue", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_02",
        category: "korean_possessive_plus_demonstrative",
        setup: &[Turn {
            text: "캐시를 검사하고 큐를 수리해.",
            language: Korean,
        }],
        follow: Turn {
            text: "그것의 상태를 검사하고 그 대상을 분석해.",
            language: Korean,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("큐", 2)],
            forbidden_markers: &["그것의", "그 대상"],
            selected_antecedents: &[("큐", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_03",
        category: "english_repeated_possessive",
        setup: &[Turn {
            text: "Inspect the manifest.",
            language: English,
        }],
        follow: Turn {
            text: "Compare its status with its checksum.",
            language: English,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("manifest", 2)],
            forbidden_markers: &["its"],
            selected_antecedents: &[("manifest", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_04",
        category: "korean_repeated_possessive",
        setup: &[Turn {
            text: "매니페스트를 검사해.",
            language: Korean,
        }],
        follow: Turn {
            text: "그것의 상태와 그것의 체크섬을 비교해.",
            language: Korean,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("매니페스트", 2)],
            forbidden_markers: &["그것의"],
            selected_antecedents: &[("매니페스트", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_05",
        category: "english_distinct_demonstratives",
        setup: &[Turn {
            text: "Repair the snapshot.",
            language: English,
        }],
        follow: Turn {
            text: "Inspect that object, then analyze that item.",
            language: English,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("snapshot", 2)],
            forbidden_markers: &["that object", "that item"],
            selected_antecedents: &[("snapshot", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_06",
        category: "korean_distinct_demonstratives",
        setup: &[Turn {
            text: "스냅샷을 수리해.",
            language: Korean,
        }],
        follow: Turn {
            text: "그 대상을 검사하고 그 항목을 분석해.",
            language: Korean,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("스냅샷", 2)],
            forbidden_markers: &["그 대상", "그 항목"],
            selected_antecedents: &[("스냅샷", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_07",
        category: "english_ordered_local_anchor",
        setup: &[Turn {
            text: "Inspect the cache and repair the queue.",
            language: English,
        }],
        follow: Turn {
            text: "Inspect the former and report its status.",
            language: English,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("cache", 2)],
            forbidden_markers: &["former", "its"],
            selected_antecedents: &[("cache", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_08",
        category: "korean_ordered_local_anchor",
        setup: &[Turn {
            text: "캐시를 검사하고 큐를 수리해.",
            language: Korean,
        }],
        follow: Turn {
            text: "전자를 검사하고 그것의 상태를 보고해.",
            language: Korean,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("캐시", 2)],
            forbidden_markers: &["전자", "그것의"],
            selected_antecedents: &[("캐시", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_09",
        category: "english_person_plus_focus",
        setup: &[Turn {
            text: "Dana inspected the parser.",
            language: English,
        }],
        follow: Turn {
            text: "Summarize her report and inspect its status.",
            language: English,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("dana", 1), ("parser", 1)],
            forbidden_markers: &[" her ", "its"],
            selected_antecedents: &[("dana", 1), ("parser", 1)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_10",
        category: "english_second_person_plus_focus",
        setup: &[Turn {
            text: "Bob analyzed the archive.",
            language: English,
        }],
        follow: Turn {
            text: "Compare his report with that object's status.",
            language: English,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("bob", 1), ("archive", 1)],
            forbidden_markers: &[" his ", "that object"],
            selected_antecedents: &[("bob", 1), ("archive", 1)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_11",
        category: "english_quoted_marker_inert",
        setup: &[Turn {
            text: "Repair the queue.",
            language: English,
        }],
        follow: Turn {
            text: "The label says ‘its status’; inspect that object.",
            language: English,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("queue", 1)],
            forbidden_markers: &["that object"],
            selected_antecedents: &[("queue", 1)],
            minimum_mentions: 2,
            minimum_bindings: 1,
        },
    },
    Case {
        id: "R58_12",
        category: "korean_quoted_marker_inert",
        setup: &[Turn {
            text: "큐를 수리해.",
            language: Korean,
        }],
        follow: Turn {
            text: "표시에는 ‘그것의 상태’라고 적혀 있어. 그 대상을 검사해.",
            language: Korean,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("큐", 1)],
            forbidden_markers: &["그 대상"],
            selected_antecedents: &[("큐", 1)],
            minimum_mentions: 2,
            minimum_bindings: 1,
        },
    },
    Case {
        id: "R58_13",
        category: "english_multiple_missing_antecedents",
        setup: &[],
        follow: Turn {
            text: "Inspect its status, then analyze that object.",
            language: English,
        },
        expectation: Expectation::Unresolved {
            live_markers: &["its", "that object"],
            minimum_mentions: 2,
            minimum_unresolved: 2,
        },
    },
    Case {
        id: "R58_14",
        category: "korean_multiple_missing_antecedents",
        setup: &[],
        follow: Turn {
            text: "그것의 상태를 검사하고 그 대상을 분석해.",
            language: Korean,
        },
        expectation: Expectation::Unresolved {
            live_markers: &["그것의", "그 대상"],
            minimum_mentions: 2,
            minimum_unresolved: 2,
        },
    },
];

fn main() {
    emit(
        "R58_REFERENCE_RESOLUTION_COMPOSITION_DIAGNOSTIC",
        CASES,
        false,
    );
}
