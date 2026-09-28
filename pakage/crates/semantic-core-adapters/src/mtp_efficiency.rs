//! B_Core-conditioned temporal speculation policy.
//!
//! The selector consumes only typed response-plan state known before language
//! realization. Surface text and keyword matches are intentionally absent.

use serde::{Deserialize, Serialize};

use crate::natural_realization::{NaturalResponseActIR, NaturalResponsePlanIR};

pub const MTP_SPECULATION_DECISION_SCHEMA: &str = "B_CORE_MTP_SPECULATION_DECISION_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MtpSpeculationModeIR {
    Off,
    Draft2,
    Draft4,
}

impl MtpSpeculationModeIR {
    pub fn draft_max(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Draft2 => 2,
            Self::Draft4 => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtpConditioningSignalsIR {
    /// A bounded estimate emitted by response planning before wording begins.
    pub expected_response_tokens: u16,
    /// Confidence of the selected conclusion after opposing evidence is kept.
    pub conclusion_certainty_millis: u16,
}

impl MtpConditioningSignalsIR {
    pub fn validate(self) -> bool {
        self.expected_response_tokens > 0
            && self.expected_response_tokens <= 4_096
            && self.conclusion_certainty_millis <= 1_000
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MtpSelectionReasonIR {
    ShortExpectedResponse,
    ShortResponseType,
    MediumExpectedResponse,
    UncertainConclusion,
    LongEvidenceBoundResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtpSpeculationDecisionIR {
    pub schema: &'static str,
    pub mode: MtpSpeculationModeIR,
    pub draft_max: u8,
    pub expected_response_tokens: u16,
    pub response_type: NaturalResponseActIR,
    pub conclusion_certainty_millis: u16,
    pub evidence_item_count: usize,
    pub reasons: Vec<MtpSelectionReasonIR>,
    pub surface_text_consulted: bool,
}

impl MtpSpeculationDecisionIR {
    pub fn validate(&self) -> bool {
        self.schema == MTP_SPECULATION_DECISION_SCHEMA
            && self.draft_max == self.mode.draft_max()
            && self.expected_response_tokens > 0
            && self.expected_response_tokens <= 4_096
            && self.conclusion_certainty_millis <= 1_000
            && self.evidence_item_count > 0
            && !self.reasons.is_empty()
            && !self.surface_text_consulted
    }
}

fn short_response_type(response_type: NaturalResponseActIR) -> bool {
    matches!(
        response_type,
        NaturalResponseActIR::InformAcknowledgement
            | NaturalResponseActIR::SocialBackchannel
            | NaturalResponseActIR::HoldFloor
    )
}

/// Selects MTP OFF, draft-max 2, or draft-max 4 before realization.
///
/// The thresholds are deliberately small and auditable. They are POC policy
/// constants to be replaced only by held-out latency/quality evidence.
pub fn select_mtp_speculation(
    response_plan: &NaturalResponsePlanIR,
    signals: MtpConditioningSignalsIR,
) -> Option<MtpSpeculationDecisionIR> {
    if !response_plan.validate() || !signals.validate() {
        return None;
    }

    let response_type = response_plan.primary_act();
    let evidence_item_count = response_plan
        .moves
        .iter()
        .map(|response_move| response_move.evidence.len())
        .sum::<usize>();
    if evidence_item_count == 0 {
        return None;
    }

    let (mode, reasons) = if signals.expected_response_tokens <= 18 {
        (
            MtpSpeculationModeIR::Off,
            vec![MtpSelectionReasonIR::ShortExpectedResponse],
        )
    } else if short_response_type(response_type) {
        (
            MtpSpeculationModeIR::Off,
            vec![MtpSelectionReasonIR::ShortResponseType],
        )
    } else if signals.conclusion_certainty_millis < 850 {
        (
            MtpSpeculationModeIR::Draft2,
            vec![MtpSelectionReasonIR::UncertainConclusion],
        )
    } else if signals.expected_response_tokens >= 40 && evidence_item_count >= 2 {
        (
            MtpSpeculationModeIR::Draft4,
            vec![MtpSelectionReasonIR::LongEvidenceBoundResponse],
        )
    } else {
        (
            MtpSpeculationModeIR::Draft2,
            vec![MtpSelectionReasonIR::MediumExpectedResponse],
        )
    };

    let decision = MtpSpeculationDecisionIR {
        schema: MTP_SPECULATION_DECISION_SCHEMA,
        mode,
        draft_max: mode.draft_max(),
        expected_response_tokens: signals.expected_response_tokens,
        response_type,
        conclusion_certainty_millis: signals.conclusion_certainty_millis,
        evidence_item_count,
        reasons,
        surface_text_consulted: false,
    };
    debug_assert!(decision.validate());
    Some(decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::natural_realization::{
        NaturalResponseFormatIR, NaturalResponseMoveIR, NaturalResponseMoveRoleIR,
    };

    fn plan(acts: &[NaturalResponseActIR]) -> NaturalResponsePlanIR {
        let primary_move_index = acts.len() - 1;
        NaturalResponsePlanIR {
            moves: acts
                .iter()
                .enumerate()
                .map(|(index, response_act)| NaturalResponseMoveIR {
                    move_index: index,
                    role: if index == primary_move_index {
                        NaturalResponseMoveRoleIR::PrimaryTask
                    } else {
                        NaturalResponseMoveRoleIR::RequiredContent
                    },
                    response_act: *response_act,
                    evidence: vec![format!("EVIDENCE-{index}")],
                    semantic_authority: false,
                    external_action_executed: false,
                })
                .collect(),
            primary_move_index,
            response_format: NaturalResponseFormatIR::Plain,
            semantic_authority: false,
            language_can_execute: false,
        }
    }

    #[test]
    fn short_response_turns_mtp_off_without_surface_text() {
        let response_plan = plan(&[NaturalResponseActIR::TemporalAnswer]);
        let decision = select_mtp_speculation(
            &response_plan,
            MtpConditioningSignalsIR {
                expected_response_tokens: 14,
                conclusion_certainty_millis: 1_000,
            },
        )
        .unwrap();
        assert_eq!(decision.mode, MtpSpeculationModeIR::Off);
        assert!(!decision.surface_text_consulted);
        assert!(decision.validate());
    }

    #[test]
    fn medium_response_uses_two_token_draft() {
        let response_plan = plan(&[NaturalResponseActIR::PlanResultStatus]);
        let decision = select_mtp_speculation(
            &response_plan,
            MtpConditioningSignalsIR {
                expected_response_tokens: 30,
                conclusion_certainty_millis: 940,
            },
        )
        .unwrap();
        assert_eq!(decision.mode, MtpSpeculationModeIR::Draft2);
    }

    #[test]
    fn long_certain_evidence_bound_response_uses_four_token_draft() {
        let response_plan = plan(&[
            NaturalResponseActIR::TemporalAnswer,
            NaturalResponseActIR::DialogueRelationAnswer,
            NaturalResponseActIR::DiscourseAnswer,
            NaturalResponseActIR::PlanResultStatus,
        ]);
        let decision = select_mtp_speculation(
            &response_plan,
            MtpConditioningSignalsIR {
                expected_response_tokens: 64,
                conclusion_certainty_millis: 950,
            },
        )
        .unwrap();
        assert_eq!(decision.mode, MtpSpeculationModeIR::Draft4);
        assert_eq!(decision.evidence_item_count, 4);
    }

    #[test]
    fn uncertainty_caps_long_response_at_two_token_draft() {
        let response_plan = plan(&[
            NaturalResponseActIR::TemporalAnswer,
            NaturalResponseActIR::PlanResultStatus,
        ]);
        let decision = select_mtp_speculation(
            &response_plan,
            MtpConditioningSignalsIR {
                expected_response_tokens: 64,
                conclusion_certainty_millis: 700,
            },
        )
        .unwrap();
        assert_eq!(decision.mode, MtpSpeculationModeIR::Draft2);
    }
}
