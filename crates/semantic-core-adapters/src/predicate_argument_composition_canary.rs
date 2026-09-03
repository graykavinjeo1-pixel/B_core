//! Frozen R53 diagnostic for predicate/argument coordination composition.
//!
//! The suite distinguishes one predicate over a coordinated argument set,
//! coordinated predicates sharing one argument set, and coordinated predicates
//! with explicitly different arguments. The language graph remains
//! non-authoritative even when the grammatical structure is recoverable.

mod predicate_argument_composition_canary_support;

use predicate_argument_composition_canary_support::{
    emit, ArgumentExpectation as Arg, Case, FrameExpectation as Frame,
};
use semantic_core_adapters::{QuantifierKindIR as Quantifier, SemanticRoleKindIR as Role};

const EN_TWO: &[Arg] = &[
    Arg::new(Role::Theme, "cache", None),
    Arg::new(Role::CoTheme, "index", None),
];
const EN_THREE: &[Arg] = &[
    Arg::new(Role::Theme, "cache", None),
    Arg::new(Role::CoTheme, "index", None),
    Arg::new(Role::CoTheme, "manifest", None),
];
const EN_QUANTIFIED: &[Arg] = &[
    Arg::new(Role::Theme, "cache", Some(Quantifier::All)),
    Arg::new(Role::CoTheme, "index", Some(Quantifier::Each)),
];
const EN_PASSIVE: &[Arg] = &[
    Arg::new(Role::Patient, "cache", None),
    Arg::new(Role::CoTheme, "index", None),
];
const EN_COMPARE: &[Arg] = &[
    Arg::new(Role::Theme, "cache", None),
    Arg::new(Role::ComparisonPeer, "index", None),
    Arg::new(Role::ComparisonPeer, "manifest", None),
];
const EN_CACHE: &[Arg] = &[Arg::new(Role::Theme, "cache", None)];
const EN_INDEX: &[Arg] = &[Arg::new(Role::Theme, "index", None)];
const EN_COMPOUND: &[Arg] = &[Arg::new(Role::Theme, "research and development plan", None)];

const KO_TWO: &[Arg] = &[
    Arg::new(Role::CoTheme, "캐시", None),
    Arg::new(Role::Theme, "인덱스", None),
];
const KO_THREE: &[Arg] = &[
    Arg::new(Role::CoTheme, "캐시", None),
    Arg::new(Role::CoTheme, "인덱스", None),
    Arg::new(Role::Theme, "매니페스트", None),
];
const KO_QUANTIFIED: &[Arg] = &[
    Arg::new(Role::CoTheme, "캐시", Some(Quantifier::All)),
    Arg::new(Role::Theme, "인덱스", Some(Quantifier::Each)),
];

const F_EN_TWO: &[Frame] = &[Frame::new("INVESTIGATE", EN_TWO, true)];
const F_EN_THREE: &[Frame] = &[Frame::new("INVESTIGATE", EN_THREE, true)];
const F_EN_QUANTIFIED: &[Frame] = &[Frame::new("INVESTIGATE", EN_QUANTIFIED, true)];
const F_EN_PASSIVE: &[Frame] = &[Frame::new("INVESTIGATE", EN_PASSIVE, false)];
const F_EN_COMPARE: &[Frame] = &[Frame::new("INVESTIGATE", EN_COMPARE, true)];
const F_EN_CROSS: &[Frame] = &[
    Frame::new("INVESTIGATE", EN_TWO, true),
    Frame::new("REPAIR", EN_TWO, true),
];
const F_EN_DISTINCT: &[Frame] = &[
    Frame::new("INVESTIGATE", EN_CACHE, true),
    Frame::new("REPAIR", EN_INDEX, true),
];
const F_EN_QUOTED: &[Frame] = &[Frame::new("INVESTIGATE", EN_TWO, false)];
const F_EN_COMPOUND: &[Frame] = &[Frame::new("INVESTIGATE", EN_COMPOUND, true)];

const F_KO_TWO: &[Frame] = &[Frame::new("INVESTIGATE", KO_TWO, true)];
const F_KO_THREE: &[Frame] = &[Frame::new("INVESTIGATE", KO_THREE, true)];
const F_KO_QUANTIFIED: &[Frame] = &[Frame::new("INVESTIGATE", KO_QUANTIFIED, true)];
const F_KO_CROSS: &[Frame] = &[
    Frame::new("INVESTIGATE", KO_TWO, true),
    Frame::new("REPAIR", KO_TWO, true),
];

const CASES: &[Case] = &[
    Case::new(
        "R53_01",
        "english_two_argument_coordination",
        "Inspect the cache and the index.",
        F_EN_TWO,
        0,
        0,
    ),
    Case::new(
        "R53_02",
        "english_three_argument_coordination",
        "Inspect the cache, the index, and the manifest.",
        F_EN_THREE,
        0,
        0,
    ),
    Case::new(
        "R53_03",
        "english_predicate_argument_cross_product",
        "Inspect and repair the cache and the index.",
        F_EN_CROSS,
        2,
        2,
    ),
    Case::new(
        "R53_04",
        "english_quantified_argument_coordination",
        "Inspect every cache and each index.",
        F_EN_QUANTIFIED,
        0,
        0,
    ),
    Case::new(
        "R53_05",
        "english_passive_argument_coordination",
        "The cache and the index were inspected.",
        F_EN_PASSIVE,
        0,
        0,
    ),
    Case::new(
        "R53_06",
        "english_prepositional_peer_coordination",
        "Compare the cache with the index and the manifest.",
        F_EN_COMPARE,
        0,
        0,
    ),
    Case::new(
        "R53_07",
        "korean_two_argument_coordination",
        "캐시와 인덱스를 점검해.",
        F_KO_TWO,
        0,
        0,
    ),
    Case::new(
        "R53_08",
        "korean_three_argument_coordination",
        "캐시와 인덱스와 매니페스트를 점검해.",
        F_KO_THREE,
        0,
        0,
    ),
    Case::new(
        "R53_09",
        "korean_predicate_argument_cross_product",
        "캐시와 인덱스를 점검하고 수리해.",
        F_KO_CROSS,
        2,
        2,
    ),
    Case::new(
        "R53_10",
        "korean_quantified_argument_coordination",
        "모든 캐시와 각 인덱스를 점검해.",
        F_KO_QUANTIFIED,
        0,
        0,
    ),
    Case::new(
        "R53_11",
        "explicit_distinct_predicate_arguments",
        "Inspect the cache and repair the index.",
        F_EN_DISTINCT,
        0,
        2,
    ),
    Case::new(
        "R53_12",
        "quoted_coordination_has_no_authority",
        "The guide says ‘inspect the cache and the index.’",
        F_EN_QUOTED,
        0,
        0,
    ),
    Case::new(
        "R53_13",
        "lexical_compound_is_not_silently_split",
        "Inspect the research and development plan.",
        F_EN_COMPOUND,
        0,
        0,
    ),
];

fn main() {
    emit("R53_PREDICATE_ARGUMENT_COMPOSITION_DIAGNOSTIC", CASES);
}
