//! Shared Korean copula morphology for realization and interpretation.
//!
//! The module returns grammatical form evidence.  Callers remain responsible
//! for deciding whether a clause is a fact, question, condition, or quotation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KoreanCopulaFormIR {
    Dictionary,
    PlainStatement,
    InformalStatement,
    PoliteStatement,
    FormalStatement,
    PlainQuestion,
    PoliteQuestion,
    FormalQuestion,
    Conditional,
    Coordinate,
    Attributive,
    EmbeddedQuestion,
    RememberInformal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KoreanCopulaAnalysisIR<'a> {
    pub root: &'a str,
    pub polarity: bool,
    pub form: KoreanCopulaFormIR,
    /// The surface particle or coda-sensitive ending disagreed with the
    /// registered root's Hangul coda.  Meaning analysis may proceed while the
    /// original source remains available and realization emits the canonical
    /// form.
    pub particle_repair: bool,
}

pub(crate) fn positive_suffix(form: KoreanCopulaFormIR, final_coda: bool) -> &'static str {
    use KoreanCopulaFormIR as F;
    match form {
        F::Dictionary => "이다",
        F::PlainStatement => "다",
        F::InformalStatement => {
            if final_coda {
                "이야"
            } else {
                "야"
            }
        }
        F::PoliteStatement => {
            if final_coda {
                "이에요"
            } else {
                "예요"
            }
        }
        F::FormalStatement => "입니다",
        F::PlainQuestion => "인가",
        F::PoliteQuestion => "인가요",
        F::FormalQuestion => "입니까",
        F::Conditional => "이면",
        F::Coordinate => "이고",
        F::Attributive => "인",
        F::EmbeddedQuestion => "인지",
        F::RememberInformal => {
            if final_coda {
                "이구나"
            } else {
                "구나"
            }
        }
    }
}

pub(crate) fn negative_components(
    form: KoreanCopulaFormIR,
    final_coda: bool,
) -> (&'static str, &'static str) {
    use KoreanCopulaFormIR as F;
    let particle = if final_coda { "이" } else { "가" };
    let auxiliary = match form {
        F::Dictionary | F::PlainStatement => "아니다",
        F::InformalStatement | F::RememberInformal => "아니야",
        F::PoliteStatement => "아니에요",
        F::FormalStatement => "아닙니다",
        F::PlainQuestion => "아닌가",
        F::PoliteQuestion => "아닌가요",
        F::FormalQuestion => "아닙니까",
        F::Conditional => "아니면",
        F::Coordinate => "아니고",
        F::Attributive => "아닌",
        F::EmbeddedQuestion => "아닌지",
    };
    (particle, auxiliary)
}

#[cfg(test)]
pub(crate) fn realize(root: &str, form: KoreanCopulaFormIR, polarity: bool) -> Option<String> {
    if root.trim().is_empty() {
        return None;
    }
    let coda = crate::korean_nominal::surface_coda(root).unwrap_or(false);
    if polarity {
        Some(format!("{root}{}", positive_suffix(form, coda)))
    } else {
        let (particle, auxiliary) = negative_components(form, coda);
        Some(format!("{root}{particle} {auxiliary}"))
    }
}

fn needs_particle_repair(root: &str, observed_coda: bool) -> bool {
    crate::korean_nominal::surface_coda(root).is_some_and(|actual| actual != observed_coda)
}

pub(crate) fn analyze(predicate: &str) -> Option<KoreanCopulaAnalysisIR<'_>> {
    use KoreanCopulaFormIR as F;

    for (auxiliary, form) in [
        ("아닙니다", F::FormalStatement),
        ("아닙니까", F::FormalQuestion),
        ("아입니더", F::FormalStatement),
        ("아입니꺼", F::FormalQuestion),
        ("아니에요", F::PoliteStatement),
        ("아니에유", F::PoliteStatement),
        ("아닌가요", F::PoliteQuestion),
        ("아닌가유", F::PoliteQuestion),
        ("아닌가", F::PlainQuestion),
        ("아니면", F::Conditional),
        ("아니고", F::Coordinate),
        ("아닌지", F::EmbeddedQuestion),
        ("아닌", F::Attributive),
        ("아니야", F::InformalStatement),
        ("아니다", F::Dictionary),
    ] {
        let Some(nominal) = predicate
            .strip_suffix(auxiliary)
            .and_then(|value| value.strip_suffix(' '))
        else {
            continue;
        };
        for (particle, observed_coda) in [('이', true), ('가', false)] {
            if let Some(root) = nominal.strip_suffix(particle) {
                if !root.trim().is_empty() {
                    return Some(KoreanCopulaAnalysisIR {
                        root,
                        polarity: false,
                        form,
                        particle_repair: needs_particle_repair(root, observed_coda),
                    });
                }
            }
        }
    }

    // A malformed negative particle must not fall through and be reclassified
    // as a positive plain statement merely because its auxiliary ends in 다.
    if predicate
        .rsplit_once(' ')
        .is_some_and(|(_, tail)| tail.starts_with("아니") || tail.starts_with("아닙"))
    {
        return None;
    }

    for (ending, form, coda) in [
        ("입니다", F::FormalStatement, None),
        ("입니까", F::FormalQuestion, None),
        ("입니더", F::FormalStatement, None),
        ("입니꺼", F::FormalQuestion, None),
        ("인가요", F::PoliteQuestion, None),
        ("인가유", F::PoliteQuestion, None),
        ("이에요", F::PoliteStatement, Some(true)),
        ("예요", F::PoliteStatement, Some(false)),
        ("이에유", F::PoliteStatement, Some(true)),
        ("예유", F::PoliteStatement, Some(false)),
        ("이구나", F::RememberInformal, Some(true)),
        ("구나", F::RememberInformal, Some(false)),
        ("이야", F::InformalStatement, Some(true)),
        ("야", F::InformalStatement, Some(false)),
        ("이다", F::Dictionary, None),
        ("이면", F::Conditional, None),
        ("이고", F::Coordinate, None),
        ("인지", F::EmbeddedQuestion, None),
        ("인가", F::PlainQuestion, None),
        ("인", F::Attributive, None),
        ("다", F::PlainStatement, None),
    ] {
        if let Some(root) = predicate.strip_suffix(ending) {
            if !root.trim().is_empty() {
                return Some(KoreanCopulaAnalysisIR {
                    root,
                    polarity: true,
                    form,
                    particle_repair: coda
                        .is_some_and(|observed_coda| needs_particle_repair(root, observed_coda)),
                });
            }
        }
    }
    None
}

pub(crate) fn analyze_attributive(predicate: &str) -> Option<KoreanCopulaAnalysisIR<'_>> {
    analyze(predicate).filter(|analysis| analysis.form == KoreanCopulaFormIR::Attributive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_copulas_roundtrip_for_both_coda_classes() {
        use KoreanCopulaFormIR as F;
        for root in ["사람", "나무"] {
            for form in [
                F::Dictionary,
                F::PlainStatement,
                F::InformalStatement,
                F::PoliteStatement,
                F::FormalStatement,
                F::PlainQuestion,
                F::PoliteQuestion,
                F::FormalQuestion,
                F::Conditional,
                F::Coordinate,
                F::Attributive,
                F::EmbeddedQuestion,
                F::RememberInformal,
            ] {
                for polarity in [true, false] {
                    let surface = realize(root, form, polarity).unwrap();
                    let analysis = analyze(&surface).unwrap_or_else(|| panic!("{surface}"));
                    assert_eq!(analysis.root, root, "{surface}");
                    assert_eq!(analysis.polarity, polarity, "{surface}");
                    assert!(!analysis.particle_repair, "{surface}");
                    if polarity || !matches!(form, F::PlainStatement | F::RememberInformal) {
                        assert_eq!(analysis.form, form, "{surface}");
                    }
                }
            }
        }
    }

    #[test]
    fn observed_particle_can_ground_unknown_label_sound_without_guessing() {
        use KoreanCopulaFormIR as F;

        let consonant = analyze("Qx이 아닙니다").unwrap();
        let vowel = analyze("Qx가 아닙니다").unwrap();
        assert!(!consonant.polarity && !vowel.polarity);
        assert_eq!(consonant.root, "Qx");
        assert_eq!(vowel.root, "Qx");
        assert!(!consonant.particle_repair && !vowel.particle_repair);

        for root in ["사람", "나무"] {
            let final_coda = crate::korean_nominal::spelled_coda(root).unwrap();
            for form in [
                F::Dictionary,
                F::PlainStatement,
                F::InformalStatement,
                F::PoliteStatement,
                F::FormalStatement,
                F::PlainQuestion,
                F::PoliteQuestion,
                F::FormalQuestion,
                F::Conditional,
                F::Coordinate,
                F::Attributive,
                F::EmbeddedQuestion,
                F::RememberInformal,
            ] {
                let (_, auxiliary) = negative_components(form, final_coda);
                let wrong_particle = if final_coda { "가" } else { "이" };
                let malformed = format!("{root}{wrong_particle} {auxiliary}");
                let analysis = analyze(&malformed).unwrap_or_else(|| panic!("{malformed}"));
                assert!(analysis.particle_repair, "{malformed}");
                assert_eq!(
                    realize(analysis.root, analysis.form, analysis.polarity),
                    realize(root, form, false),
                    "{malformed}"
                );
            }

            for form in [
                F::InformalStatement,
                F::PoliteStatement,
                F::RememberInformal,
            ] {
                let malformed = format!("{root}{}", positive_suffix(form, !final_coda));
                let analysis = analyze(&malformed).unwrap_or_else(|| panic!("{malformed}"));
                assert!(analysis.particle_repair, "{malformed}");
                assert_eq!(
                    realize(analysis.root, analysis.form, analysis.polarity),
                    realize(root, form, true),
                    "{malformed}"
                );
            }
        }
    }

    #[test]
    fn numeric_copulas_follow_korean_digit_readings() {
        use KoreanCopulaFormIR as F;
        for (root, polite, casual) in [
            ("0", "0이에요", "0이야"),
            ("1", "1이에요", "1이야"),
            ("2", "2예요", "2야"),
            ("6", "6이에요", "6이야"),
            ("8", "8이에요", "8이야"),
            ("9", "9예요", "9야"),
        ] {
            assert_eq!(
                realize(root, F::PoliteStatement, true).as_deref(),
                Some(polite)
            );
            assert_eq!(
                realize(root, F::InformalStatement, true).as_deref(),
                Some(casual)
            );
            assert!(!analyze(polite).unwrap().particle_repair, "{polite}");
            assert!(!analyze(casual).unwrap().particle_repair, "{casual}");
        }
        assert!(analyze("1예요").unwrap().particle_repair);
        assert!(analyze("2이에요").unwrap().particle_repair);
    }

    #[test]
    fn regional_copulas_preserve_root_polarity_and_clause_function() {
        use KoreanCopulaFormIR as F;
        for (surface, root, polarity, form) in [
            ("사람입니더", "사람", true, F::FormalStatement),
            ("나무입니꺼", "나무", true, F::FormalQuestion),
            ("사람이 아입니더", "사람", false, F::FormalStatement),
            ("나무가 아입니꺼", "나무", false, F::FormalQuestion),
            ("사람이에유", "사람", true, F::PoliteStatement),
            ("나무예유", "나무", true, F::PoliteStatement),
            ("사람이 아니에유", "사람", false, F::PoliteStatement),
            ("나무가 아닌가유", "나무", false, F::PoliteQuestion),
        ] {
            let analysis = analyze(surface).unwrap_or_else(|| panic!("{surface}"));
            assert_eq!(analysis.root, root, "{surface}");
            assert_eq!(analysis.polarity, polarity, "{surface}");
            assert_eq!(analysis.form, form, "{surface}");
            assert!(!analysis.particle_repair, "{surface}");
        }
    }
}
