//! Frozen R53 held-out transfer for predicate/argument composition.
//!
//! This suite is intentionally separate from the diagnostic cases and is
//! frozen before its first execution.

mod predicate_argument_composition_canary_support;

use predicate_argument_composition_canary_support::{
    emit, ArgumentExpectation as Arg, Case, FrameExpectation as Frame,
};
use semantic_core_adapters::{QuantifierKindIR as Quantifier, SemanticRoleKindIR as Role};

const LOG_METRIC: &[Arg] = &[
    Arg::new(Role::Theme, "logs", None),
    Arg::new(Role::CoTheme, "metrics", None),
];
const REPORT_SET: &[Arg] = &[
    Arg::new(Role::Theme, "report", None),
    Arg::new(Role::CoTheme, "summary", None),
    Arg::new(Role::CoTheme, "appendix", None),
];
const MULTIWORD_SET: &[Arg] = &[
    Arg::new(Role::Theme, "source code", None),
    Arg::new(Role::CoTheme, "build manifest", None),
];
const QUANTIFIED_SET: &[Arg] = &[
    Arg::new(Role::Theme, "cache", Some(Quantifier::All)),
    Arg::new(Role::CoTheme, "index", Some(Quantifier::Each)),
];
const KOREAN_QUANTIFIED_SET: &[Arg] = &[
    Arg::new(Role::CoTheme, "로그", Some(Quantifier::All)),
    Arg::new(Role::Theme, "보고서", Some(Quantifier::Each)),
];
const INSTRUMENT_SET: &[Arg] = &[
    Arg::new(Role::Theme, "archive", None),
    Arg::new(Role::Instrument, "parser", None),
    Arg::new(Role::Instrument, "checksum tool", None),
];
const CACHE_INDEX: &[Arg] = &[
    Arg::new(Role::Theme, "cache", None),
    Arg::new(Role::CoTheme, "index", None),
];
const MANIFEST_REPORT: &[Arg] = &[
    Arg::new(Role::Theme, "manifest", None),
    Arg::new(Role::CoTheme, "report", None),
];

const F_LOG_METRIC: &[Frame] = &[Frame::new("INVESTIGATE", LOG_METRIC, true)];
const F_REPORT_SET: &[Frame] = &[Frame::new("INVESTIGATE", REPORT_SET, true)];
const F_MULTIWORD_SET: &[Frame] = &[Frame::new("INVESTIGATE", MULTIWORD_SET, true)];
const F_EN_THREE_BY_TWO: &[Frame] = &[
    Frame::new("INVESTIGATE", QUANTIFIED_SET, true),
    Frame::new("REPAIR", QUANTIFIED_SET, true),
    Frame::new("INVESTIGATE", QUANTIFIED_SET, true),
];
const F_KO_THREE_BY_TWO: &[Frame] = &[
    Frame::new("INVESTIGATE", KOREAN_QUANTIFIED_SET, true),
    Frame::new("REPAIR", KOREAN_QUANTIFIED_SET, true),
    Frame::new("CREATE", KOREAN_QUANTIFIED_SET, true),
];
const F_INSTRUMENT_SET: &[Frame] = &[Frame::new("INVESTIGATE", INSTRUMENT_SET, true)];
const F_QUOTED_CROSS: &[Frame] = &[
    Frame::new("INVESTIGATE", LOG_METRIC, false),
    Frame::new("REPAIR", LOG_METRIC, false),
];
const F_DISTINCT_GROUPS: &[Frame] = &[
    Frame::new("INVESTIGATE", CACHE_INDEX, true),
    Frame::new("REPAIR", MANIFEST_REPORT, true),
];

const CASES: &[Case] = &[
    Case::new(
        "R53_H01",
        "english_bare_noun_coordination",
        "Verify logs and metrics.",
        F_LOG_METRIC,
        0,
        0,
    ),
    Case::new(
        "R53_H02",
        "english_mixed_comma_coordination",
        "Review the report, summary and appendix.",
        F_REPORT_SET,
        0,
        0,
    ),
    Case::new(
        "R53_H03",
        "english_multiword_member_coordination",
        "Analyze the source code and the build manifest.",
        F_MULTIWORD_SET,
        0,
        0,
    ),
    Case::new(
        "R53_H04",
        "english_three_predicates_by_two_arguments",
        "Check, restore, and verify every cache and each index.",
        F_EN_THREE_BY_TWO,
        4,
        3,
    ),
    Case::new(
        "R53_H05",
        "korean_three_predicates_by_two_arguments",
        "모든 로그와 각 보고서를 검토하고 수정하고 문서화해.",
        F_KO_THREE_BY_TWO,
        4,
        3,
    ),
    Case::new(
        "R53_H06",
        "english_coordinated_instruments",
        "Inspect the archive using the parser and the checksum tool.",
        F_INSTRUMENT_SET,
        0,
        0,
    ),
    Case::new(
        "R53_H07",
        "quoted_predicate_argument_cross_product",
        "The note says ‘check and repair logs and metrics.’",
        F_QUOTED_CROSS,
        2,
        0,
    ),
    Case::new(
        "R53_H08",
        "explicit_distinct_argument_groups",
        "Inspect the cache and the index, then repair the manifest and the report.",
        F_DISTINCT_GROUPS,
        0,
        2,
    ),
];

fn main() {
    emit("R53_PREDICATE_ARGUMENT_COMPOSITION_HELDOUT", CASES);
}
