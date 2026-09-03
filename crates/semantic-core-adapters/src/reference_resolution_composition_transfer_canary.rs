//! Frozen R58 held-out transfer suite for compositional reference resolution.

mod reference_resolution_composition_canary_support;

use reference_resolution_composition_canary_support::{emit, Case, Expectation, Turn};
use semantic_core_adapters::LanguageCodeIR::{English, Korean};

const CASES: &[Case<'_>] = &[
    Case {
        id: "R58_H01",
        category: "english_possessive_demonstrative_transfer",
        setup: &[Turn {
            text: "Analyze the index.",
            language: English,
        }],
        follow: Turn {
            text: "Verify its shape and inspect that item.",
            language: English,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("index", 2)],
            forbidden_markers: &["its", "that item"],
            selected_antecedents: &[("index", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_H02",
        category: "korean_possessive_demonstrative_transfer",
        setup: &[Turn {
            text: "인덱스를 분석해.",
            language: Korean,
        }],
        follow: Turn {
            text: "그것의 형태를 검증하고 그 항목을 검사해.",
            language: Korean,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("인덱스", 2)],
            forbidden_markers: &["그것의", "그 항목"],
            selected_antecedents: &[("인덱스", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_H03",
        category: "english_repeated_possessive_transfer",
        setup: &[Turn {
            text: "Inspect the ledger.",
            language: English,
        }],
        follow: Turn {
            text: "Compare its owner with its timestamp.",
            language: English,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("ledger", 2)],
            forbidden_markers: &["its"],
            selected_antecedents: &[("ledger", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_H04",
        category: "korean_repeated_possessive_transfer",
        setup: &[Turn {
            text: "원장을 검사해.",
            language: Korean,
        }],
        follow: Turn {
            text: "그것의 소유자와 그것의 시각을 비교해.",
            language: Korean,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("원장", 2)],
            forbidden_markers: &["그것의"],
            selected_antecedents: &[("원장", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_H05",
        category: "english_latter_local_anchor_transfer",
        setup: &[Turn {
            text: "Inspect the parser and repair the index.",
            language: English,
        }],
        follow: Turn {
            text: "Analyze the latter and summarize its state.",
            language: English,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("index", 2)],
            forbidden_markers: &["latter", "its"],
            selected_antecedents: &[("index", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_H06",
        category: "korean_latter_local_anchor_transfer",
        setup: &[Turn {
            text: "파서를 검사하고 인덱스를 수리해.",
            language: Korean,
        }],
        follow: Turn {
            text: "후자를 분석하고 그것의 상태를 요약해.",
            language: Korean,
        },
        expectation: Expectation::Resolved {
            surfaces: &[("인덱스", 2)],
            forbidden_markers: &["후자", "그것의"],
            selected_antecedents: &[("인덱스", 2)],
            minimum_mentions: 2,
            minimum_bindings: 2,
        },
    },
    Case {
        id: "R58_H07",
        category: "english_multiple_missing_transfer",
        setup: &[],
        follow: Turn {
            text: "Verify its owner and inspect that item.",
            language: English,
        },
        expectation: Expectation::Unresolved {
            live_markers: &["its", "that item"],
            minimum_mentions: 2,
            minimum_unresolved: 2,
        },
    },
    Case {
        id: "R58_H08",
        category: "korean_multiple_missing_transfer",
        setup: &[],
        follow: Turn {
            text: "그것의 소유자를 검증하고 그 항목을 검사해.",
            language: Korean,
        },
        expectation: Expectation::Unresolved {
            live_markers: &["그것의", "그 항목"],
            minimum_mentions: 2,
            minimum_unresolved: 2,
        },
    },
];

fn main() {
    emit("R58_REFERENCE_RESOLUTION_COMPOSITION_HELDOUT", CASES, true);
}
