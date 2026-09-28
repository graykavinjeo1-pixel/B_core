//! Shared Korean 하다 morphology for realization and interpretation.
//!
//! These forms are language evidence only.  An analyzed future, question, or
//! connective does not by itself authorize a present-tense world fact.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KoreanHadaFormIR {
    Dictionary,
    PlainStatement,
    InformalStatement,
    PoliteStatement,
    FormalStatement,
    PoliteQuestion,
    FormalQuestion,
    PlainActionQuestion,
    PlainStateQuestion,
    PoliteStateQuestion,
    Conditional,
    Coordinate,
    StateAttributive,
    ActionAttributive,
    StateEmbeddedQuestion,
    ActionEmbeddedQuestion,
    EvidentialPlain,
    EvidentialPolite,
    ReasonPlain,
    ReasonPolite,
    SharedKnowledgePlain,
    SharedKnowledgePolite,
    RememberInformal,
    FutureInformal,
    FutureFormal,
    NecessityPolite,
    InviteInformal,
    InviteFormal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KoreanHadaAnalysisIR {
    pub polarity: bool,
    pub form: KoreanHadaFormIR,
}

/// Return the material following the nominal stem.  Punctuation belongs to the
/// clause planner, so the same form inventory can be used while parsing input.
pub(crate) fn suffix(form: KoreanHadaFormIR, polarity: bool) -> Option<&'static str> {
    use KoreanHadaFormIR as F;
    match (form, polarity) {
        (F::Dictionary, true) => Some("하다"),
        (F::Dictionary, false) => Some("하지 않다"),
        (F::PlainStatement, true) => Some("한다"),
        (F::PlainStatement, false) => Some("하지 않는다"),
        (F::InformalStatement, true) => Some("해"),
        (F::InformalStatement, false) => Some("하지 않아"),
        (F::PoliteStatement, true) => Some("해요"),
        (F::PoliteStatement, false) => Some("하지 않아요"),
        (F::FormalStatement, true) => Some("합니다"),
        (F::FormalStatement, false) => Some("하지 않습니다"),
        (F::PoliteQuestion, true) => Some("하나요"),
        (F::PoliteQuestion, false) => Some("하지 않나요"),
        (F::FormalQuestion, true) => Some("합니까"),
        (F::FormalQuestion, false) => Some("하지 않습니까"),
        (F::PlainActionQuestion, true) => Some("하는가"),
        (F::PlainActionQuestion, false) => Some("하지 않는가"),
        (F::PlainStateQuestion, true) => Some("한가"),
        (F::PlainStateQuestion, false) => Some("하지 않은가"),
        (F::PoliteStateQuestion, true) => Some("한가요"),
        (F::PoliteStateQuestion, false) => Some("하지 않은가요"),
        (F::Conditional, true) => Some("하면"),
        (F::Conditional, false) => Some("하지 않으면"),
        (F::Coordinate, true) => Some("하고"),
        (F::Coordinate, false) => Some("하지 않고"),
        (F::StateAttributive, true) => Some("한"),
        (F::StateAttributive, false) => Some("하지 않은"),
        (F::ActionAttributive, true) => Some("하는"),
        (F::ActionAttributive, false) => Some("하지 않는"),
        (F::StateEmbeddedQuestion, true) => Some("한지"),
        (F::StateEmbeddedQuestion, false) => Some("하지 않은지"),
        (F::ActionEmbeddedQuestion, true) => Some("하는지"),
        (F::ActionEmbeddedQuestion, false) => Some("하지 않는지"),
        (F::EvidentialPlain, true) => Some("하네"),
        (F::EvidentialPlain, false) => Some("하지 않네"),
        (F::EvidentialPolite, true) => Some("하네요"),
        (F::EvidentialPolite, false) => Some("하지 않네요"),
        (F::ReasonPlain, true) => Some("하거든"),
        (F::ReasonPlain, false) => Some("하지 않거든"),
        (F::ReasonPolite, true) => Some("하거든요"),
        (F::ReasonPolite, false) => Some("하지 않거든요"),
        (F::SharedKnowledgePlain, true) => Some("하잖아"),
        (F::SharedKnowledgePlain, false) => Some("하지 않잖아"),
        (F::SharedKnowledgePolite, true) => Some("하잖아요"),
        (F::SharedKnowledgePolite, false) => Some("하지 않잖아요"),
        (F::RememberInformal, true) => Some("하구나"),
        (F::RememberInformal, false) => Some("하지 않구나"),
        (F::FutureInformal, true) => Some("할게"),
        (F::FutureInformal, false) => Some("하지 않을게"),
        (F::FutureFormal, true) => Some("하겠습니다"),
        (F::FutureFormal, false) => Some("하지 않겠습니다"),
        (F::NecessityPolite, true) => Some("해야 해요"),
        (F::NecessityPolite, false) => Some("하지 않아야 해요"),
        (F::InviteInformal, true) => Some("해 보자"),
        (F::InviteFormal, true) => Some("해 봅시다"),
        (F::InviteInformal | F::InviteFormal, false) => None,
    }
}

pub(crate) fn realize(
    nominal_stem: &str,
    form: KoreanHadaFormIR,
    polarity: bool,
) -> Option<String> {
    (!nominal_stem.trim().is_empty())
        .then(|| suffix(form, polarity).map(|tail| format!("{nominal_stem}{tail}")))
        .flatten()
}

pub(crate) fn negative_components(form: KoreanHadaFormIR) -> Option<(&'static str, &'static str)> {
    let tail = suffix(form, false)?;
    tail.split_once(' ')
}

fn finite_analysis(tail: &str) -> Option<KoreanHadaAnalysisIR> {
    use KoreanHadaFormIR as F;
    let (polarity, form) = match tail {
        "하다" => (true, F::Dictionary),
        "하지 않다" => (false, F::Dictionary),
        "한다" => (true, F::PlainStatement),
        "하지 않는다" => (false, F::PlainStatement),
        "해" => (true, F::InformalStatement),
        "하지 않아" => (false, F::InformalStatement),
        "해요" => (true, F::PoliteStatement),
        "하지 않아요" => (false, F::PoliteStatement),
        "합니다" => (true, F::FormalStatement),
        "하지 않습니다" => (false, F::FormalStatement),
        "합니더" => (true, F::FormalStatement),
        "하지 않습니더" => (false, F::FormalStatement),
        "하나요" => (true, F::PoliteQuestion),
        "하지 않나요" => (false, F::PoliteQuestion),
        "합니까" => (true, F::FormalQuestion),
        "하지 않습니까" => (false, F::FormalQuestion),
        "합니꺼" => (true, F::FormalQuestion),
        "하지 않습니꺼" => (false, F::FormalQuestion),
        "해유" => (true, F::PoliteStatement),
        "하지 않아유" => (false, F::PoliteStatement),
        "하나유" => (true, F::PoliteQuestion),
        "하지 않나유" => (false, F::PoliteQuestion),
        "하는가" => (true, F::PlainActionQuestion),
        "하지 않는가" => (false, F::PlainActionQuestion),
        "하면" => (true, F::Conditional),
        "하지 않으면" => (false, F::Conditional),
        "하고" => (true, F::Coordinate),
        "하지 않고" => (false, F::Coordinate),
        "하네" => (true, F::EvidentialPlain),
        "하지 않네" => (false, F::EvidentialPlain),
        "하네요" => (true, F::EvidentialPolite),
        "하지 않네요" => (false, F::EvidentialPolite),
        "하네유" => (true, F::EvidentialPolite),
        "하지 않네유" => (false, F::EvidentialPolite),
        "하거든" => (true, F::ReasonPlain),
        "하지 않거든" => (false, F::ReasonPlain),
        "하거든요" => (true, F::ReasonPolite),
        "하지 않거든요" => (false, F::ReasonPolite),
        "하거든유" => (true, F::ReasonPolite),
        "하지 않거든유" => (false, F::ReasonPolite),
        "하잖아" => (true, F::SharedKnowledgePlain),
        "하지 않잖아" => (false, F::SharedKnowledgePlain),
        "하잖아요" => (true, F::SharedKnowledgePolite),
        "하지 않잖아요" => (false, F::SharedKnowledgePolite),
        "하잖아유" => (true, F::SharedKnowledgePolite),
        "하지 않잖아유" => (false, F::SharedKnowledgePolite),
        _ => return None,
    };
    Some(KoreanHadaAnalysisIR { polarity, form })
}

pub(crate) fn analyze_finite_tail(
    tail: &str,
    allow_state_question: bool,
) -> Option<KoreanHadaAnalysisIR> {
    finite_analysis(tail).or_else(|| {
        allow_state_question
            .then(|| analyze_state_question_tail(tail))
            .flatten()
    })
}

pub(crate) fn analyze_state_question_tail(tail: &str) -> Option<KoreanHadaAnalysisIR> {
    use KoreanHadaFormIR as F;
    let (polarity, form) = match tail {
        "한가" => (true, F::PlainStateQuestion),
        "한가요" => (true, F::PoliteStateQuestion),
        "한가유" => (true, F::PoliteStateQuestion),
        "하지 않은가" => (false, F::PlainStateQuestion),
        "하지 않은가요" => (false, F::PoliteStateQuestion),
        "하지 않은가유" => (false, F::PoliteStateQuestion),
        _ => return None,
    };
    Some(KoreanHadaAnalysisIR { polarity, form })
}

pub(crate) fn analyze_attributive_tail(tail: &str, state: bool) -> Option<KoreanHadaAnalysisIR> {
    use KoreanHadaFormIR as F;
    let (polarity, form) = match (tail, state) {
        ("한", true) => (true, F::StateAttributive),
        ("하지 않은", true) => (false, F::StateAttributive),
        ("하는", false) => (true, F::ActionAttributive),
        ("하지 않는", false) => (false, F::ActionAttributive),
        _ => return None,
    };
    Some(KoreanHadaAnalysisIR { polarity, form })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_current_forms_are_interpretable_by_the_same_inventory() {
        use KoreanHadaFormIR as F;
        for form in [
            F::Dictionary,
            F::PlainStatement,
            F::InformalStatement,
            F::PoliteStatement,
            F::FormalStatement,
            F::PoliteQuestion,
            F::FormalQuestion,
            F::PlainActionQuestion,
            F::Conditional,
            F::Coordinate,
            F::EvidentialPlain,
            F::EvidentialPolite,
            F::ReasonPlain,
            F::ReasonPolite,
            F::SharedKnowledgePlain,
            F::SharedKnowledgePolite,
        ] {
            for polarity in [true, false] {
                let surface = realize("점검", form, polarity).unwrap();
                let analysis =
                    analyze_finite_tail(surface.strip_prefix("점검").unwrap(), false).unwrap();
                assert_eq!(analysis, KoreanHadaAnalysisIR { polarity, form });
            }
        }
        assert!(analyze_finite_tail("합니다", false)
            .is_some_and(|analysis| analysis.form == F::FormalStatement));
        assert!(analyze_finite_tail("하나요", false)
            .is_some_and(|analysis| analysis.form == F::PoliteQuestion));
    }

    #[test]
    fn state_forms_roundtrip_without_becoming_action_forms() {
        use KoreanHadaFormIR as F;
        for (form, polite) in [
            (F::PlainStateQuestion, false),
            (F::PoliteStateQuestion, true),
        ] {
            for polarity in [true, false] {
                let surface = realize("분주", form, polarity).unwrap();
                let analysis = analyze_state_question_tail(
                    surface.strip_prefix("분주").expect("same lexical stem"),
                )
                .unwrap();
                assert_eq!(analysis.polarity, polarity);
                assert_eq!(analysis.form == F::PoliteStateQuestion, polite);
            }
        }
        for (state, form) in [(true, F::StateAttributive), (false, F::ActionAttributive)] {
            for polarity in [true, false] {
                let surface = realize("분주", form, polarity).unwrap();
                assert_eq!(
                    analyze_attributive_tail(surface.strip_prefix("분주").unwrap(), state),
                    Some(KoreanHadaAnalysisIR { polarity, form })
                );
                assert!(
                    analyze_attributive_tail(surface.strip_prefix("분주").unwrap(), !state)
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn future_and_invitation_realizations_do_not_parse_as_current_facts() {
        use KoreanHadaFormIR as F;
        for form in [
            F::FutureInformal,
            F::FutureFormal,
            F::NecessityPolite,
            F::InviteInformal,
            F::InviteFormal,
        ] {
            let surface = realize("점검", form, true).unwrap();
            assert!(analyze_finite_tail(surface.strip_prefix("점검").unwrap(), true).is_none());
        }
    }

    #[test]
    fn regional_finite_forms_share_the_standard_semantic_analysis() {
        use KoreanHadaFormIR as F;
        for (tail, polarity, form) in [
            ("합니더", true, F::FormalStatement),
            ("하지 않습니더", false, F::FormalStatement),
            ("합니꺼", true, F::FormalQuestion),
            ("하지 않습니꺼", false, F::FormalQuestion),
            ("해유", true, F::PoliteStatement),
            ("하지 않아유", false, F::PoliteStatement),
            ("하나유", true, F::PoliteQuestion),
            ("하지 않나유", false, F::PoliteQuestion),
            ("하거든유", true, F::ReasonPolite),
            ("하지 않잖아유", false, F::SharedKnowledgePolite),
        ] {
            assert_eq!(
                analyze_finite_tail(tail, false),
                Some(KoreanHadaAnalysisIR { polarity, form }),
                "{tail}"
            );
        }
    }
}
