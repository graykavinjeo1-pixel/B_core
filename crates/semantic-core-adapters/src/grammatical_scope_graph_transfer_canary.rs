//! Frozen R56 first-exposure transfer suite for grammatical scope.
//!
//! These examples use predicates, entities, restrictions, and word orders not
//! present in the diagnostic suite. The expectations are structural and were
//! frozen before this binary was executed.

mod grammatical_scope_graph_canary_support;

use grammatical_scope_graph_canary_support::{emit, Case, KindCount};

fn cases() -> Vec<Case> {
    vec![
        case(
            "R56_H01",
            "english_novel_restriction_conjunction",
            "Restore each archive that is encrypted and not expired.",
            Some("EACH"),
            &[
                ("QUANTIFIER", 1),
                ("RESTRICTION", 2),
                ("CONJUNCTION", 1),
                ("NEGATION", 1),
            ],
            0,
            1,
            1,
            0,
            1,
            None,
        ),
        case(
            "R56_H02",
            "korean_novel_restriction_conjunction",
            "손상됐고 잠겨 있지 않은 모든 인덱스를 복구해",
            Some("ALL"),
            &[
                ("QUANTIFIER", 1),
                ("RESTRICTION", 2),
                ("CONJUNCTION", 1),
                ("NEGATION", 1),
            ],
            0,
            1,
            1,
            0,
            1,
            None,
        ),
        case(
            "R56_H03",
            "english_novel_restriction_disjunction",
            "Review every report that includes tables or lacks citations.",
            Some("ALL"),
            &[("QUANTIFIER", 1), ("RESTRICTION", 2), ("DISJUNCTION", 1)],
            0,
            1,
            1,
            0,
            1,
            None,
        ),
        case(
            "R56_H04",
            "korean_novel_restriction_disjunction",
            "오류가 있거나 서명이 없는 각 문서를 검토해",
            Some("EACH"),
            &[("QUANTIFIER", 1), ("RESTRICTION", 2), ("DISJUNCTION", 1)],
            0,
            1,
            1,
            0,
            1,
            None,
        ),
        case(
            "R56_H05",
            "english_novel_negation_quantifier_ambiguity",
            "Do not remove each stale record.",
            Some("EACH"),
            &[("QUANTIFIER", 1), ("NEGATION", 1), ("RESTRICTION", 1)],
            1,
            1,
            0,
            1,
            0,
            None,
        ),
        case(
            "R56_H06",
            "korean_novel_none_scope",
            "실패한 작업은 하나도 배포하지 마",
            Some("NONE"),
            &[("QUANTIFIER", 1), ("NEGATION", 1), ("RESTRICTION", 1)],
            0,
            1,
            0,
            1,
            0,
            None,
        ),
        case(
            "R56_H07",
            "english_novel_focus_only_restriction",
            "Inspect only workers that exceeded the limit.",
            None,
            &[("FOCUS_ONLY", 1), ("RESTRICTION", 1)],
            0,
            1,
            1,
            0,
            1,
            None,
        ),
        case(
            "R56_H08",
            "english_novel_shared_argument_recursive_scope",
            "Analyze and document every log that is recent but not complete.",
            Some("ALL"),
            &[
                ("QUANTIFIER", 1),
                ("RESTRICTION", 2),
                ("CONJUNCTION", 1),
                ("NEGATION", 1),
            ],
            0,
            2,
            2,
            0,
            2,
            Some(true),
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
const fn case(
    id: &'static str,
    category: &'static str,
    text: &'static str,
    expected_quantifier: Option<&'static str>,
    expected_kind_minima: &'static [KindCount],
    minimum_ambiguities: usize,
    expected_frames: usize,
    expected_selected: usize,
    expected_blocked: usize,
    expected_authorized: usize,
    expect_shared_primary_argument: Option<bool>,
) -> Case {
    Case {
        id,
        category,
        text,
        expected_quantifier,
        expected_kind_minima,
        minimum_ambiguities,
        expected_frames,
        expected_selected,
        expected_blocked,
        expected_authorized,
        expect_shared_primary_argument,
    }
}

fn main() {
    emit(
        "R56_GRAMMATICAL_SCOPE_GRAPH_FIRST_EXPOSURE_TRANSFER",
        cases(),
    );
}
