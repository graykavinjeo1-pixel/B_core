//! Bounded inference from utterance form to the response goal the user is
//! pragmatically requesting.
//!
//! This graph is adapter evidence, not semantic truth.  It can ask the
//! planner for an assessment, explanation, recommendation, or diagnostic,
//! but it never authorizes an external action and never mutates a concept.

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const UTTERANCE_INTENT_GRAPH_SCHEMA: &str = "B_CORE_UTTERANCE_INTENT_GRAPH_IR_10";
pub const MAX_UTTERANCE_INTENT_SIGNALS: usize = 32;
pub const MAX_UTTERANCE_INTENT_CANDIDATES: usize = 8;

/// Missing input for a decision. These are dialogue roles, not world facts or
/// task-specific solution templates. Source replay prevents a later layer from
/// replacing a requested choice with a factual lookup or an execution promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecisionInputIR {
    DesiredOutcome,
    Deadline,
    Constraints,
    Preference,
    ExpectedBenefit,
}

/// A proposed action is an evaluation target, not an asserted event or an
/// execution grant. Lexical senses remain alternatives until world evidence
/// disambiguates them; their labels do not establish the action's benefit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedActionIR {
    pub surface: String,
    pub predicate_entry_ids: Vec<String>,
    pub negated: bool,
    /// Shared event-role grammar under a PROPOSAL scope, never an observation.
    pub event: crate::proposition_content::DescribedEventIR,
    /// Dictionary senses remain alternatives. Frame compatibility is not
    /// semantic sense selection, a benefit claim, or permission to execute.
    pub frame_candidates: Vec<ProposedFrameCandidateIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedFrameCandidateIR {
    pub entry_id: String,
    pub sense_id: String,
    pub source_pattern: Option<String>,
    pub pattern_understood: bool,
    pub required_roles: Vec<crate::proposition_content::ContentSlotIR>,
    pub missing_roles: Vec<crate::proposition_content::ContentSlotIR>,
    pub incompatible_roles: Vec<crate::proposition_content::ContentSlotIR>,
}

fn proposed_frames(
    event: &crate::proposition_content::DescribedEventIR,
) -> Option<Vec<ProposedFrameCandidateIR>> {
    use crate::proposition_content::ContentSlotIR as R;
    use std::collections::BTreeSet;
    let pack = crate::lexical_knowledge_pack::builtin_pack();
    let korean = event
        .predicate_surface
        .chars()
        .any(|c| ('가'..='힣').contains(&c));
    let mut result = Vec::new();
    // Direct entry lookup only. Neither the dictionary nor the world catalogue
    // is scanned to search for a convenient sense that supports an answer.
    for id in &event.lexical_entry_ids {
        let entry = pack.entry(id)?;
        for sense in &entry.senses {
            // Korean headwords share their listed senses; English aliases are
            // attached per sense. An entry hit must not import every unrelated
            // translation of that Korean headword into an English proposal.
            // The proposal envelope requires an infinitive, not a finite form.
            if !korean
                && !sense
                    .english
                    .split(';')
                    .any(|alias| alias.trim().eq_ignore_ascii_case(&event.predicate_surface))
            {
                continue;
            }
            if result.len() >= 128 {
                return None;
            }
            let pattern = sense.grammar.get("syntacticPattern").cloned();
            let mut required = BTreeSet::new();
            let mut allowed = BTreeSet::new();
            let mut understood = pattern
                .as_ref()
                .is_some_and(|p| p.split_whitespace().any(|w| w == entry.lemma));
            if let Some(pattern) = &pattern {
                for word in pattern.split_whitespace() {
                    let optional = word.starts_with('(') && word.ends_with(')');
                    let token = word.trim_matches(['(', ')']);
                    if token == entry.lemma {
                        continue;
                    }
                    let suffix = token.trim_start_matches(|c: char| c.is_ascii_digit());
                    let role = match suffix {
                        "이" | "가" => Some(R::Agent),
                        "을" | "를" => Some(R::Theme),
                        "에게" | "한테" => Some(R::Recipient),
                        "에게서" | "한테서" => Some(R::Source),
                        "에서" => Some(R::Location),
                        _ => None,
                    };
                    if suffix == token || role.is_none() {
                        understood = false;
                    } else if let Some(role) = role {
                        allowed.insert(role);
                        if !optional {
                            required.insert(role);
                        }
                    }
                }
            }
            // A partially understood source pattern licenses nothing. Keep the
            // original pattern visible instead of quietly treating it as free
            // valency. Actor omission also stays a gap, not 'the tired person'.
            if !understood {
                required.clear();
                allowed.clear();
            }
            let missing_roles = required
                .iter()
                .filter(|r| !event.roles.contains_key(r))
                .copied()
                .collect();
            let incompatible_roles = if understood {
                event
                    .roles
                    .keys()
                    .filter(|r| {
                        !allowed.contains(r) && !matches!(r, R::Location | R::Time | R::Duration)
                    })
                    .copied()
                    .collect()
            } else {
                vec![]
            };
            result.push(ProposedFrameCandidateIR {
                entry_id: id.clone(),
                sense_id: sense.source_sense_id.clone(),
                source_pattern: pattern,
                pattern_understood: understood,
                required_roles: required.into_iter().collect(),
                missing_roles,
                incompatible_roles,
            });
        }
    }
    (!result.is_empty()).then_some(result)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionInquiryIR {
    pub source_text: String,
    pub missing_input: DecisionInputIR,
    pub continues_context: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_action: Option<ProposedActionIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation_of: Option<DecisionInquiryOriginIR>,
    /// When present, the requested expected-benefit input has a conditional,
    /// core-derived answer. Kept in the same dialogue owner, not a new route.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assessment: Option<Box<crate::world_dialogue::ActionBenefitAssessmentIR>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge_gap: Option<Box<crate::world_dialogue::ActionBenefitGapIR>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resumption: Option<Box<crate::world_dialogue::ActionBenefitResumptionIR>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clarification_reply: Option<Box<DecisionClarificationReplyIR>>,
    /// User-supplied decision conditions are dialogue evidence, not asserted
    /// world state.  They remain bound to the unanswered question so a later
    /// choice can use the same stated criteria without promoting them to facts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_evidence: Vec<DecisionContextEvidenceIR>,
    /// Declarative context supplied in the same user turn as the decision
    /// question.  It is provenance-bound dialogue context, never a world
    /// observation or a surface template.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inline_context: Vec<DecisionInlineContextIR>,
    /// A source-bound choice conclusion.  It exists only when one offered
    /// alternative has a uniquely supported structural feature match with the
    /// retained decision context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice_selection: Option<DecisionChoiceSelectionIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionContextEvidenceIR {
    pub source_text: String,
    pub source_sha256: String,
    pub original_question_source: String,
    pub question_source_sha256: String,
    pub question_turn: u64,
    pub turn: u64,
    pub input_kind: DecisionInputIR,
}

impl DecisionContextEvidenceIR {
    fn validate_for(&self, inquiry: &DecisionInquiryIR) -> bool {
        self.question_turn > 0
            && self.turn > self.question_turn
            && self.turn - self.question_turn <= 3
            && self.input_kind == inquiry.missing_input
            && self.question_source_sha256 == decision_source_sha256(&self.original_question_source)
            && self.source_sha256 == decision_source_sha256(&self.source_text)
            && !self.source_text.trim().is_empty()
            && self.source_text.chars().count() <= 2048
            && !self.source_text.contains(['?', '？', '"', '“', '”', '‘', '’', '`', '\n', ';'])
            && decision_inquiry(&self.source_text).is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionInlineContextIR {
    pub source_text: String,
    pub source_sha256: String,
}

impl DecisionInlineContextIR {
    fn validate_for(&self, inquiry: &DecisionInquiryIR) -> bool {
        !self.source_text.trim().is_empty()
            && self.source_text.chars().count() <= 2048
            && self.source_sha256 == decision_source_sha256(&self.source_text)
            && inquiry.source_text.contains(&self.source_text)
            && !self.source_text.contains(['?', '？', '"', '“', '”', '‘', '’', '`', '\n', ';'])
            && decision_inquiry(&self.source_text).is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionChoiceOptionIR {
    pub source_text: String,
    pub source_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionChoiceSelectionIR {
    pub question_source: String,
    pub question_source_sha256: String,
    pub options: Vec<DecisionChoiceOptionIR>,
    pub selected_option_index: usize,
    pub matching_features: Vec<String>,
}

impl DecisionChoiceSelectionIR {
    fn validate_for(&self, inquiry: &DecisionInquiryIR) -> bool {
        let question_source = inquiry
            .context_evidence
            .last()
            .map(|context| context.original_question_source.as_str())
            .unwrap_or(inquiry.source_text.as_str());
        self.question_source == question_source
            && self.question_source_sha256 == decision_source_sha256(question_source)
            && (2..=4).contains(&self.options.len())
            && self.selected_option_index < self.options.len()
            && self.options.iter().all(|option| {
                !option.source_text.trim().is_empty()
                    && option.source_text.chars().count() <= 256
                    && option.source_sha256 == decision_source_sha256(&option.source_text)
                    && self.question_source.contains(&option.source_text)
            })
            && self.options.iter().enumerate().all(|(index, option)| {
                self.options[..index]
                    .iter()
                    .all(|prior| prior.source_sha256 != option.source_sha256)
            })
            && !self.matching_features.is_empty()
            && self.matching_features.len() <= 4
            && self.matching_features.iter().all(|feature| {
                feature.chars().count() >= 2
                    && feature.chars().count() <= 32
                    && self.options[self.selected_option_index]
                        .source_text
                        .contains(feature)
            })
            && self.matching_features.windows(2).all(|pair| pair[0] < pair[1])
            && decision_choice_selection(
                question_source,
                &decision_context_sources(inquiry),
            )
            .as_ref()
                == Some(self)
    }
}

/// A response to an emitted information request, not an observation of the
/// world. Retains a flat question origin; followups never extend its lifetime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionClarificationReplyIR {
    pub original_source: String,
    pub question_turn: u64,
    pub turn: u64,
    pub kind: crate::world_dialogue::WorldClarificationFollowupKindIR,
    /// Explicit asking predicates refer to the original question; bare why
    /// after an abstention refers to the latest response-choice act.
    pub explains_question: bool,
    pub abstention: Option<crate::world_dialogue::WorldClarificationAbstentionIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionInquiryOriginIR {
    pub source_text: String,
    pub asked_turn: u64,
}

impl DecisionInquiryIR {
    pub fn validate(&self) -> bool {
        if self.inline_context.len() > 3
            || !self
                .inline_context
                .iter()
                .all(|context| context.validate_for(self))
            || self
                .inline_context
                .iter()
                .enumerate()
                .any(|(index, context)| self.inline_context[..index]
                    .iter()
                    .any(|prior| prior.source_sha256 == context.source_sha256))
            || !self
                .choice_selection
                .as_ref()
                .is_none_or(|selection| selection.validate_for(self))
            || self.context_evidence.len() > 3
            || !self
                .context_evidence
                .iter()
                .all(|evidence| evidence.validate_for(self))
            || !self
                .context_evidence
                .windows(2)
                .all(|pair| pair[0].turn < pair[1].turn)
        {
            return false;
        }
        if let Some(last_context) = self.context_evidence.last() {
            let Some(original) = decision_inquiry(&last_context.original_question_source) else {
                return false;
            };
            let is_condition_reply = self.source_text == last_context.source_text;
            let is_continued_question = decision_inquiry(&self.source_text).is_some_and(|next| {
                next.continues_context
                    && next.missing_input == original.missing_input
                    && next.proposed_action == original.proposed_action
            });
            return (is_condition_reply || is_continued_question)
                && self.missing_input == original.missing_input
                && (is_condition_reply || self.continues_context)
                && self.proposed_action == original.proposed_action
                && self.inline_context == original.inline_context
                && self.choice_selection.as_ref().is_none_or(|selection|
                    selection.question_source == last_context.original_question_source)
                && self.explanation_of.is_none()
                && self.assessment.is_none()
                && self.knowledge_gap.is_none()
                && self.resumption.is_none()
                && self.clarification_reply.is_none();
        }
        if let Some(reply) = &self.clarification_reply {
            use crate::world_dialogue::WorldClarificationFollowupKindIR as K;
            let Some(original) = decision_inquiry(&reply.original_source) else {
                return false;
            };
            let Some(gap) = &self.knowledge_gap else {
                return false;
            };
            let mut base = original.clone();
            base.knowledge_gap = self.knowledge_gap.clone();
            return base.validate()
                && self.assessment.is_none()
                && self.resumption.is_none()
                && self.proposed_action == original.proposed_action
                && self.missing_input == original.missing_input
                && self.continues_context == original.continues_context
                && reply.question_turn > 0
                && reply.question_turn == gap.evaluated_turn
                && reply.turn > reply.question_turn
                && reply.turn - reply.question_turn <= 3
                && gap_requests_information(gap)
                && reply.explains_question
                    == (reply.kind == K::ReasonRequest
                        && (reply.abstention.is_none()
                            || explicit_question_reason(&self.source_text)))
                && (crate::world_dialogue::clarification_followup(&self.source_text)
                    == Some(reply.kind)
                    || (reply.kind == K::ReasonRequest
                        && decision_reason_request(&self.source_text)))
                && self
                    .explanation_of
                    .as_ref()
                    .map_or(reply.kind != K::ReasonRequest, |o| {
                        reply.kind == K::ReasonRequest
                            && o.source_text == reply.original_source
                            && o.asked_turn == reply.question_turn
                    })
                && reply.abstention.as_ref().is_none_or(|a| {
                    matches!(a.kind, K::UnknownAnswer | K::DeclinedAnswer)
                        && a.turn > reply.question_turn
                        && a.turn <= reply.turn
                        && crate::world_dialogue::clarification_followup(&a.source_text)
                            == Some(a.kind)
                })
                && (!matches!(reply.kind, K::UnknownAnswer | K::DeclinedAnswer)
                    || reply.abstention.as_ref().is_some_and(|a| {
                        a.kind == reply.kind
                            && a.source_text == self.source_text
                            && a.turn == reply.turn
                    }));
        }
        if let Some(resumption) = &self.resumption {
            let Some(original) = decision_inquiry(&resumption.original_source) else {
                return false;
            };
            let Some(action) = &self.proposed_action else {
                return false;
            };
            return resumption.validate()
                && self.missing_input == original.missing_input
                && self.proposed_action == original.proposed_action
                && if let Some(origin) = &self.explanation_of {
                    decision_reason_request(&self.source_text)
                        && self.continues_context
                        && origin.source_text == resumption.original_source
                        && origin.asked_turn == resumption.question_turn
                } else {
                    self.source_text == resumption.update.source_text
                        && self.continues_context == original.continues_context
                }
                && match (&self.assessment, &self.knowledge_gap) {
                    (Some(a), None) => {
                        a.current_clarification
                            && a.evaluated_turn == resumption.update.turn
                            && a.matches_world(&resumption.update.memory)
                            && a.validate(action, original.continues_context)
                    }
                    (None, Some(g)) => {
                        g.current_clarification
                            && g.evaluated_turn == resumption.update.turn
                            && g.matches_world(&resumption.update.memory)
                            && g.validate(action, original.continues_context)
                    }
                    _ => false,
                };
        }
        let original_continuation = self
            .explanation_of
            .as_ref()
            .and_then(|o| decision_inquiry(&o.source_text))
            .map_or(self.continues_context, |i| i.continues_context);
        if let Some(gap) = &self.knowledge_gap {
            let mut request = self.clone();
            request.knowledge_gap = None;
            return !gap.current_clarification
                && self.assessment.is_none()
                && request.validate()
                && self.missing_input == DecisionInputIR::ExpectedBenefit
                && self
                    .proposed_action
                    .as_ref()
                    .is_some_and(|action| gap.validate(action, original_continuation));
        }
        if let Some(assessment) = &self.assessment {
            let mut request = self.clone();
            request.assessment = None;
            return !assessment.current_clarification
                && request.validate()
                && self.missing_input == DecisionInputIR::ExpectedBenefit
                && self
                    .proposed_action
                    .as_ref()
                    .is_some_and(|action| assessment.validate(action, original_continuation));
        }
        if let Some(origin) = &self.explanation_of {
            return origin.asked_turn > 0
                && decision_reason_request(&self.source_text)
                && decision_inquiry(&origin.source_text).is_some_and(|prior| {
                    prior.missing_input == self.missing_input
                        && prior.proposed_action == self.proposed_action
                })
                && self.continues_context;
        }
        let Some(mut original) = decision_inquiry(&self.source_text) else {
            return false;
        };
        original.context_evidence = self.context_evidence.clone();
        original.inline_context = self.inline_context.clone();
        original.choice_selection = self.choice_selection.clone();
        original == *self
    }

    /// Bind a declarative response to the current decision question without
    /// treating it as an observation, a completed event, or an action grant.
    pub(crate) fn with_context_evidence(
        source: &str,
        prior: &Self,
        question_turn: u64,
        turn: u64,
    ) -> Option<Self> {
        if !prior.validate()
            || question_turn == 0
            || turn <= question_turn
            || turn - question_turn > 3
            || prior.resumption.is_some()
            || prior.clarification_reply.is_some()
            || source.trim().is_empty()
            || source.split_whitespace().count() < 2
            || decision_inquiry(source).is_some()
            || source.contains(['?', '？', '"', '“', '”', '‘', '’', '`', '\n', ';'])
        {
            return None;
        }
        let original_question_source = prior
            .context_evidence
            .last()
            .map(|evidence| evidence.original_question_source.clone())
            .unwrap_or_else(|| prior.source_text.clone());
        let mut result = prior.clone();
        result.source_text = source.to_string();
        result.context_evidence.push(DecisionContextEvidenceIR {
            source_text: source.to_string(),
            source_sha256: decision_source_sha256(source),
            original_question_source: original_question_source.clone(),
            question_source_sha256: decision_source_sha256(&original_question_source),
            question_turn,
            turn,
            input_kind: prior.missing_input,
        });
        // A choice is resolved only from a feature stated in the source-bound
        // decision context and a feature literally present in one option of
        // the original question. No option property is inferred here.
        result.choice_selection = decision_choice_selection(
            &original_question_source,
            &decision_context_sources(&result),
        );
        result.validate().then_some(result)
    }

    /// Carries previously supplied decision conditions into a deictic follow-up
    /// question.  The follow-up must itself parse as a continuation of the
    /// same typed decision input; no condition is promoted to world knowledge.
    pub(crate) fn continue_with_context(
        source: &str,
        prior: &Self,
        turn: u64,
    ) -> Option<Self> {
        if !prior.validate() || prior.context_evidence.is_empty() {
            return None;
        }
        let mut next = decision_inquiry(source)?;
        if !next.continues_context
            || next.missing_input != prior.missing_input
            || next.proposed_action != prior.proposed_action
            || turn == 0
        {
            return None;
        }
        let last = prior.context_evidence.last()?;
        if turn <= last.turn || turn - last.turn > 3 {
            return None;
        }
        next.context_evidence = prior.context_evidence.clone();
        next.inline_context = prior.inline_context.clone();
        next.choice_selection = decision_choice_selection(
            &last.original_question_source,
            &decision_context_sources(&next),
        );
        next.validate().then_some(next)
    }

    pub(crate) fn explain_from(
        source: &str,
        prior: &Self,
        answered_turn: u64,
        turn: u64,
    ) -> Option<Self> {
        if answered_turn == 0 || turn <= answered_turn || turn - answered_turn > 3 {
            return None;
        }
        if let Some(reply) = Self::clarification_from(source, prior, turn) {
            return Some(reply);
        }
        if !prior.validate()
            || !decision_reason_request(source)
            || answered_turn == 0
            || turn <= answered_turn
            || turn - answered_turn > 3
        {
            return None;
        }
        let origin = prior.explanation_of.clone().unwrap_or_else(|| {
            prior.resumption.as_ref().map_or_else(
                || DecisionInquiryOriginIR {
                    source_text: prior.source_text.clone(),
                    asked_turn: answered_turn,
                },
                |r| DecisionInquiryOriginIR {
                    source_text: r.original_source.clone(),
                    asked_turn: r.question_turn,
                },
            )
        });
        if turn <= origin.asked_turn || turn - origin.asked_turn > 3 {
            return None;
        }
        Some(Self {
            source_text: source.into(),
            continues_context: true,
            missing_input: prior.missing_input,
            proposed_action: prior.proposed_action.clone(),
            explanation_of: Some(origin),
            assessment: prior.assessment.clone(),
            knowledge_gap: prior.knowledge_gap.clone(),
            resumption: prior.resumption.clone(),
            clarification_reply: None,
            context_evidence: Vec::new(),
            inline_context: Vec::new(),
            choice_selection: None,
        })
    }

    fn clarification_from(source: &str, prior: &Self, turn: u64) -> Option<Self> {
        use crate::world_dialogue::WorldClarificationFollowupKindIR as K;
        if !prior.validate() || prior.resumption.is_some() {
            return None;
        }
        let gap = prior.knowledge_gap.as_ref()?;
        if !gap_requests_information(gap)
            || turn <= gap.evaluated_turn
            || turn - gap.evaluated_turn > 3
        {
            return None;
        }
        let kind = crate::world_dialogue::clarification_followup(source)
            .or_else(|| decision_reason_request(source).then_some(K::ReasonRequest))?;
        let original_source = prior
            .clarification_reply
            .as_ref()
            .map(|r| &r.original_source)
            .or_else(|| prior.explanation_of.as_ref().map(|o| &o.source_text))
            .unwrap_or(&prior.source_text)
            .clone();
        let original = decision_inquiry(&original_source)?;
        let abstention = if matches!(kind, K::UnknownAnswer | K::DeclinedAnswer) {
            Some(crate::world_dialogue::WorldClarificationAbstentionIR {
                source_text: source.into(),
                turn,
                kind,
            })
        } else {
            prior
                .clarification_reply
                .as_ref()
                .and_then(|r| r.abstention.clone())
        };
        let mut result = original;
        result.source_text = source.into();
        result.knowledge_gap = Some(gap.clone());
        if kind == K::ReasonRequest {
            result.explanation_of = Some(DecisionInquiryOriginIR {
                source_text: original_source.clone(),
                asked_turn: gap.evaluated_turn,
            });
        }
        result.clarification_reply = Some(Box::new(DecisionClarificationReplyIR {
            original_source,
            question_turn: gap.evaluated_turn,
            turn,
            kind,
            explains_question: kind == K::ReasonRequest
                && (abstention.is_none() || explicit_question_reason(source)),
            abstention,
        }));
        result.validate().then_some(result)
    }

    pub(crate) fn resume_with_update(
        prior: &Self,
        update: crate::world_dialogue::WorldMemoryUpdateIR,
    ) -> Option<Self> {
        if !prior.validate()
            || prior.resumption.is_some()
            || prior
                .clarification_reply
                .as_ref()
                .is_some_and(|r| r.abstention.is_some())
        {
            return None;
        }
        let gap = prior.knowledge_gap.as_ref()?;
        let original_source = prior
            .clarification_reply
            .as_ref()
            .map(|r| &r.original_source)
            .or_else(|| prior.explanation_of.as_ref().map(|o| &o.source_text))
            .unwrap_or(&prior.source_text)
            .clone();
        let original = decision_inquiry(&original_source)?;
        let resumption = crate::world_dialogue::ActionBenefitResumptionIR {
            original_source,
            question_turn: gap.evaluated_turn,
            prior_gap: gap.clone(),
            update: Box::new(update),
        };
        if !resumption.validate() {
            return None;
        }
        let action = original.proposed_action.as_ref()?;
        let assessment = crate::world_dialogue::assess_action_benefit_scoped(
            action,
            original.continues_context,
            &resumption.update.memory,
            resumption.update.turn,
            true,
        )
        .map(Box::new);
        let knowledge_gap = if assessment.is_none() {
            crate::world_dialogue::action_benefit_gap_scoped(
                action,
                original.continues_context,
                &resumption.update.memory,
                resumption.update.turn,
                true,
            )
            .map(Box::new)
        } else {
            None
        };
        let result = Self {
            source_text: resumption.update.source_text.clone(),
            assessment,
            knowledge_gap,
            resumption: Some(Box::new(resumption)),
            ..original
        };
        result.validate().then_some(result)
    }

    pub(crate) fn evaluation_turn(&self, current_turn: u64) -> u64 {
        if let Some(reply) = &self.clarification_reply {
            return reply.question_turn;
        }
        self.resumption.as_ref().map_or_else(
            || {
                self.explanation_of
                    .as_ref()
                    .map_or(current_turn, |o| o.asked_turn)
            },
            |r| r.update.turn,
        )
    }
}

fn gap_requests_information(gap: &crate::world_dialogue::ActionBenefitGapIR) -> bool {
    use crate::world_dialogue::ActionBenefitGapReasonIR as G;
    !gap.current_clarification
        && matches!(
            gap.reason,
            G::Actor | G::CurrentState | G::ConflictingState | G::RecentState
        )
}

fn explicit_question_reason(source: &str) -> bool {
    crate::world_dialogue::clarification_reason_request(source)
        && source.split_whitespace().count() > 1
}

/// A causal question referring to the interlocutor's preceding dialogue act,
/// not a cause attributed to a world event or a named third party.
fn decision_reason_request(source: &str) -> bool {
    if source.chars().count() > 256 || source.contains(['"', '“', '”', '`', '\n', ';']) {
        return false;
    }
    if crate::world_dialogue::clarification_reason_request(source) {
        return true;
    }
    let normalized = source
        .trim()
        .trim_end_matches(['?', '.', '!'])
        .to_lowercase();
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    if matches!(words.as_slice(), ["why", "do", "you", "think", "that"]) {
        return true;
    }
    let Some((predicate, prefix)) = words.split_last() else {
        return false;
    };
    if !matches!(
        prefix,
        ["왜", "그렇게"] | ["왜", "너는", "그렇게"] | ["왜", "넌", "그렇게"]
    ) {
        return false;
    }
    let lexical = crate::lexical_knowledge_pack::builtin_pack().lookup(predicate);
    !lexical.truncated
        && lexical
            .matches
            .iter()
            .any(|m| m.entry.lemma == "생각하다" && !m.morphology.grammar_rule.contains("PAST"))
}

/// Controlled interrogative/imperative composition. Question word + deliberative
/// mood is required; quoted, reported, historical and compound requests are not
/// consumed by this bounded path. Unknown content is not interpreted as a fact.
pub(crate) fn decision_inquiry(source: &str) -> Option<DecisionInquiryIR> {
    if source.chars().count() > 2048 || source.contains(['"', '“', '”', '‘', '’', '`', '\n', ';'])
    {
        return None;
    }
    // A user may establish a situation and then ask one terminal question in
    // the same turn. The prior declarative material is context for that
    // question, not a second action to execute. Analyze only the final
    // interrogative unit while retaining the entire source in the sealed
    // inquiry so replay still detects any tampering with the context.
    let source = source.trim();
    let units = source
        .split(['.', '!', '?', '。', '？'])
        .map(str::trim)
        .filter(|unit| !unit.is_empty())
        .collect::<Vec<_>>();
    // Natural dialogue can put the decision request first and append its
    // relevant situation afterwards.  If exactly one independently bounded
    // unit is a decision inquiry, retain the complete source as its evidence
    // instead of dropping to a generic acknowledgement solely because the
    // request is not sentence-final.  Multiple decision units remain
    // deliberately unresolved: selecting one would discard a user request.
    if units.len() > 1 && source.matches(['?', '？']).count() <= 1 {
        let mut inquiries = units
            .iter()
            .enumerate()
            .filter_map(|(index, unit)| decision_inquiry(unit).map(|inquiry| (index, inquiry)))
            .collect::<Vec<_>>();
        if inquiries.len() == 1 {
            let (decision_index, mut inquiry) = inquiries.remove(0);
            inquiry.inline_context = units
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != decision_index)
                .map(|(_, unit)| DecisionInlineContextIR {
                    source_text: (*unit).to_string(),
                    source_sha256: decision_source_sha256(unit),
                })
                .collect();
            inquiry.source_text = source.to_string();
            inquiry.choice_selection = decision_choice_selection(
                source,
                &decision_context_sources(&inquiry),
            );
            return Some(inquiry);
        }
    }
    let terminal_source = source
        .trim_end_matches(['?', '.', '!', '？', '。'])
        .rsplit(['.', '!', '?', '。', '？'])
        .next()
        .unwrap_or(source)
        .trim();
    if terminal_source.is_empty() {
        return None;
    }
    let normalized = terminal_source.to_lowercase();
    let mut text = normalized.trim_end_matches(['?', '.', '!']).trim();
    if text.contains(['?', '.', '!']) {
        return None;
    }
    // Multi-clause requests require the full composition graph. A final advice
    // verb must never suppress an independent action elsewhere in the turn.
    if ["하고 ", "한 뒤 ", "하고나서 ", " and ", " then "]
        .iter()
        .any(|c| text.contains(c))
    {
        return None;
    }
    let mut continues_context = false;
    let mut context_prefix_bytes = 0;
    for prefix in ["그럼 ", "그러면 ", "then, ", "then ", "so, ", "so "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest;
            continues_context = true;
            context_prefix_bytes = prefix.len();
            break;
        }
    }
    let words = text.split_whitespace().collect::<Vec<_>>();
    if source.ends_with(['?', '？']) {
        // Case folding is only for the envelope. Named arguments must retain
        // their original spelling through memory, follow-up and realization.
        let original = terminal_source
            .trim_end_matches(['?', '.', '!', '？', '。'])
            .trim();
        let original_body = original.get(context_prefix_bytes..)?;
        if let Some(proposed_action) = action_evaluation_target(original_body) {
            return Some(DecisionInquiryIR {
                source_text: source.into(),
                missing_input: DecisionInputIR::ExpectedBenefit,
                continues_context,
                proposed_action: Some(proposed_action),
                explanation_of: None,
                assessment: None,
                knowledge_gap: None,
                resumption: None,
                clarification_reply: None,
                context_evidence: Vec::new(),
                inline_context: Vec::new(),
                choice_selection: None,
            });
        }
    }
    let korean_wh = words.iter().copied().find(|w| {
        matches!(
            *w,
            "언제" | "어떻게" | "뭘" | "뭐" | "무엇을" | "어느" | "어떤"
        )
    }).or_else(|| words.iter().copied().find(|word|
        ["어디가", "어디를", "어디에", "어디서", "어디로"]
            .iter()
            .any(|form| word == form)));
    let mut ko_deliberative = ["까", "까요", "지", "죠"]
        .iter()
        .any(|ending| text.ends_with(ending))
        && !["았", "었", "했", "였", "는", "인"].iter().any(|tense| {
            ["지", "죠", "을까", "을까요"]
                .iter()
                .any(|ending| text.ends_with(&format!("{tense}{ending}")))
        });
    if ko_deliberative && korean_wh.is_some() {
        // Mood alone cannot turn a recollection into a decision request.
        // Shared lexical morphology covers vowel-contracted past stems too;
        // no task verb or complete question is used as a dispatch key.
        let predicate = words.last().copied().unwrap_or_default();
        let lexical = crate::lexical_knowledge_pack::builtin_pack().lookup(predicate);
        ko_deliberative = !lexical.truncated
            && !lexical
                .matches
                .iter()
                .any(|m| m.matched_form == predicate && m.morphology.grammar_rule.contains("PAST"));
    }
    let english_wh = words.first().copied().filter(|w| {
        matches!(*w, "what" | "when" | "how" | "which")
            && matches!(words.get(1).copied(), Some("should" | "could"))
            && matches!(words.get(2).copied(), Some("i" | "we"))
            && words.len() >= 4
    });
    // A bare deliberative light verb leaves the action's content unfilled.
    // It can continue a discourse situation without an explicit "then" marker;
    // a lexical action or an added complement still names a separate target.
    continues_context |=
        english_wh.is_some() && words.len() == 4 && words.get(3).copied() == Some("do");
    let recommendation_imperative = ["추천해줘", "추천해 줘", "추천해주세요", "추천해 주세요"]
        .iter()
        .any(|ending| text.ends_with(ending))
        || text.starts_with("recommend ")
        || text.starts_with("please recommend ");
    // Permission-shaped deliberatives are requests for help choosing a course,
    // even without an overt WH word. They remain response-only: this records a
    // missing preference/constraint and never grants the proposed action.
    let korean_permission_deliberative = ["도 될까요", "도 될까", "도 되나요"]
        .iter()
        .any(|ending| text.ends_with(ending));
    let wh = korean_wh.filter(|_| ko_deliberative).or(english_wh);
    let missing_input = match wh {
        Some("언제" | "when") => DecisionInputIR::Deadline,
        Some("어떻게" | "how") => DecisionInputIR::Constraints,
        Some(_) if text.contains("하는") || text.ends_with(" do") => {
            DecisionInputIR::DesiredOutcome
        }
        Some(_) => DecisionInputIR::Preference,
        None if recommendation_imperative => DecisionInputIR::Preference,
        None if korean_permission_deliberative => DecisionInputIR::Preference,
        _ => return None,
    };
    Some(DecisionInquiryIR {
        source_text: source.to_string(),
        missing_input,
        continues_context,
        proposed_action: None,
        explanation_of: None,
        assessment: None,
        knowledge_gap: None,
        resumption: None,
        clarification_reply: None,
        context_evidence: Vec::new(),
        inline_context: Vec::new(),
        choice_selection: None,
    })
}

fn action_evaluation_target(text: &str) -> Option<ProposedActionIR> {
    // A self-deliberative modal supplies an explicit Agent, not an execution
    // grant. Parse its infinitive through the same role/sense path as an
    // evaluative proposal; no verb-specific answer or synthetic assertion.
    let modal = "should i ";
    if text
        .get(..modal.len())
        .is_some_and(|s| s.eq_ignore_ascii_case(modal))
    {
        let action = &text[modal.len()..];
        let mut proposed = infinitival_action_target(action)?;
        proposed
            .event
            .roles
            .insert(crate::proposition_content::ContentSlotIR::Agent, "I".into());
        proposed.event.event_id = format!("DESCRIBED_EVENT_{:x}", Sha256::digest(text.as_bytes()));
        proposed.frame_candidates = proposed_frames(&proposed.event)?;
        return Some(proposed);
    }
    let korean = text.chars().any(|c| ('가'..='힣').contains(&c));
    let (surface, predicate, negated) = if korean {
        let (nominal, evaluation) = text.rsplit_once(' ')?;
        let lexical = crate::lexical_knowledge_pack::builtin_pack().lookup(evaluation);
        if lexical.truncated
            || !lexical.matches.iter().any(|m| {
                m.entry.lemma == "좋다"
                    && m.entry.pos == "형용사"
                    && m.morphology.grammar_rule == "KO_STEM_MODAL_FINITE"
            })
        {
            return None;
        }
        let action = nominal
            .strip_suffix(" 게")
            .or_else(|| nominal.strip_suffix(" 것이"))?;
        let predicate = action.split_whitespace().last()?;
        // Inability is not choosing not to act.
        if action.split_whitespace().any(|w| w == "못") {
            return None;
        }
        let negated = action.split_whitespace().any(|w| w == "안");
        (format!("{action} 것"), predicate, negated)
    } else {
        let prefix = "would it be good to ";
        if !text.get(..prefix.len())?.eq_ignore_ascii_case(prefix) {
            return None;
        }
        return infinitival_action_target(&text[prefix.len()..]);
    };
    let lexical = crate::lexical_knowledge_pack::builtin_pack().lookup(predicate);
    if lexical.truncated {
        return None;
    }
    let mut predicate_entry_ids = lexical
        .matches
        .into_iter()
        .filter(|m| {
            m.entry.pos == "동사"
                && (!korean
                    || m.morphology.grammar_rule == "KO_ADNOMINAL_PRESENT"
                    || (m.morphology.grammar_rule == "SOURCE_KOREAN_PRINCIPAL_FORM"
                        && predicate.ends_with('는')))
        })
        .map(|m| m.entry.source_entry_id)
        .collect::<Vec<_>>();
    predicate_entry_ids.sort();
    predicate_entry_ids.dedup();
    if predicate_entry_ids.is_empty() || surface.split_whitespace().count() > 12 {
        return None;
    }
    let action_source = if korean {
        surface.strip_suffix(" 것")?
    } else {
        &surface
    };
    let event = crate::proposition_content::proposed_event(action_source)?;
    if event.lexical_entry_ids != predicate_entry_ids || event.negated != negated {
        return None;
    }
    let frame_candidates = proposed_frames(&event)?;
    Some(ProposedActionIR {
        surface,
        predicate_entry_ids,
        negated,
        event,
        frame_candidates,
    })
}

fn infinitival_action_target(action: &str) -> Option<ProposedActionIR> {
    if action.chars().any(|c| ('가'..='힣').contains(&c)) || action.split_whitespace().count() > 12
    {
        return None;
    }
    let (body, negated) = if action
        .get(..4)
        .is_some_and(|s| s.eq_ignore_ascii_case("not "))
    {
        (&action[4..], true)
    } else {
        (action, false)
    };
    let predicate = body.split_whitespace().next()?;
    let lexical = crate::lexical_knowledge_pack::builtin_pack().lookup(predicate);
    if lexical.truncated {
        return None;
    }
    let mut predicate_entry_ids = lexical
        .matches
        .into_iter()
        .filter(|m| m.entry.pos == "동사")
        .map(|m| m.entry.source_entry_id)
        .collect::<Vec<_>>();
    predicate_entry_ids.sort();
    predicate_entry_ids.dedup();
    let event = crate::proposition_content::proposed_event(action)?;
    if predicate_entry_ids.is_empty()
        || event.lexical_entry_ids != predicate_entry_ids
        || event.negated != negated
    {
        return None;
    }
    let frame_candidates = proposed_frames(&event)?;
    // English sense bindings accept a base-form predicate only. Empty sense
    // matches must not turn finite/past/gerund forms into licensed proposals.
    Some(ProposedActionIR {
        surface: action.into(),
        predicate_entry_ids: event.lexical_entry_ids.clone(),
        negated: event.negated,
        event,
        frame_candidates,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UtteranceSurfaceFormIR {
    Declarative,
    Interrogative,
    Imperative,
    Fragment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UtteranceSignalKindIR {
    ProblemState,
    ReadinessOrSafety,
    EvidenceDemand,
    AlternativeComparison,
    ResponseGoalCorrection,
    ExplanationDemand,
    SummaryDemand,
    ConditionalPremise,
    ContinuationDecision,
    BenefitCriterion,
    PreservationConstraint,
    InterrogativeForm,
    DeliberativeQuestion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommunicativeIntentIR {
    ProblemDisclosure,
    AssessmentRequest,
    EvidenceRequest,
    RecommendationRequest,
    ResponseGoalCorrection,
    ExplanationRequest,
    SummaryRequest,
    ConditionalDecisionRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpectedResponseKindIR {
    DiagnosisOrNextStep,
    Assessment,
    Evidence,
    Recommendation,
    /// A request to help choose, not a request to execute or report a fact.
    DecisionSupport,
    Explanation,
    Summary,
    VerifyThenDecide,
    Clarification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtteranceIntentSignalIR {
    pub signal_id: String,
    pub kind: UtteranceSignalKindIR,
    pub evidence_surface: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtteranceIntentCandidateIR {
    pub candidate_id: String,
    pub communicative_intent: CommunicativeIntentIR,
    pub expected_response: ExpectedResponseKindIR,
    pub target: String,
    pub constraints: Vec<String>,
    pub evidence_signal_ids: Vec<String>,
    pub score_millis: u16,
    pub requires_prior_context: bool,
    pub prior_context_bound: bool,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

impl UtteranceIntentCandidateIR {
    pub fn plan_intent(&self) -> Option<PlanIntentIR> {
        match self.expected_response {
            ExpectedResponseKindIR::DiagnosisOrNextStep
            | ExpectedResponseKindIR::Assessment
            | ExpectedResponseKindIR::VerifyThenDecide => Some(PlanIntentIR::Investigate),
            ExpectedResponseKindIR::Evidence
            | ExpectedResponseKindIR::Explanation
            | ExpectedResponseKindIR::Summary => Some(PlanIntentIR::Explain),
            ExpectedResponseKindIR::Recommendation => Some(PlanIntentIR::Plan),
            ExpectedResponseKindIR::Clarification | ExpectedResponseKindIR::DecisionSupport => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtteranceIntentGraphIR {
    pub schema: String,
    pub source_text_sha256: String,
    pub context_sha256: String,
    pub surface_form: UtteranceSurfaceFormIR,
    pub signals: Vec<UtteranceIntentSignalIR>,
    pub candidates: Vec<UtteranceIntentCandidateIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_id: Option<String>,
    pub unresolved_ambiguities: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
    pub graph_sha256: String,
}

impl Default for UtteranceIntentGraphIR {
    fn default() -> Self {
        analyze_utterance_intent("", None, &[])
    }
}

impl UtteranceIntentGraphIR {
    pub fn selected(&self) -> Option<&UtteranceIntentCandidateIR> {
        self.selected_candidate_id.as_ref().and_then(|selected| {
            self.candidates
                .iter()
                .find(|candidate| &candidate.candidate_id == selected)
        })
    }

    pub fn supporting(&self) -> impl Iterator<Item = &UtteranceIntentCandidateIR> {
        let selected = self.selected_candidate_id.as_deref();
        self.candidates
            .iter()
            .filter(move |candidate| Some(candidate.candidate_id.as_str()) != selected)
    }

    /// Returns the primary intent first and every compatible supporting
    /// contribution after it. All entries are evidence-only at this layer.
    pub fn active(&self) -> impl Iterator<Item = &UtteranceIntentCandidateIR> {
        self.selected().into_iter().chain(self.supporting())
    }

    pub fn requires_clarification(&self) -> bool {
        self.selected().is_some_and(|candidate| {
            candidate.expected_response == ExpectedResponseKindIR::Clarification
        }) || !self.unresolved_ambiguities.is_empty()
    }

    pub fn validate(&self) -> bool {
        if self.schema != UTTERANCE_INTENT_GRAPH_SCHEMA
            || self.source_text_sha256.len() != 64
            || self.context_sha256.len() != 64
            || self.graph_sha256.len() != 64
            || self.signals.len() > MAX_UTTERANCE_INTENT_SIGNALS
            || self.candidates.len() > MAX_UTTERANCE_INTENT_CANDIDATES
            || self.semantic_authority
            || self.external_execution_authorized
            || self.signals.iter().any(|signal| {
                signal.signal_id.is_empty()
                    || signal.byte_start > signal.byte_end
                    || signal.confidence_millis > 1_000
            })
            || self.candidates.iter().any(|candidate| {
                candidate.candidate_id.is_empty()
                    || candidate.target.trim().is_empty()
                    || candidate.score_millis > 1_000
                    || candidate.semantic_authority
                    || candidate.external_execution_authorized
            })
        {
            return false;
        }
        let signal_ids = self
            .signals
            .iter()
            .map(|signal| signal.signal_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let candidate_ids = self
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if signal_ids.len() != self.signals.len()
            || candidate_ids.len() != self.candidates.len()
            || self.candidates.iter().any(|candidate| {
                candidate
                    .evidence_signal_ids
                    .iter()
                    .any(|signal_id| !signal_ids.contains(signal_id.as_str()))
            })
            || self
                .selected_candidate_id
                .as_ref()
                .is_some_and(|selected| !candidate_ids.contains(selected.as_str()))
            || self.selected_candidate_id.as_ref().is_some_and(|selected| {
                self.candidates
                    .first()
                    .map(|candidate| &candidate.candidate_id)
                    != Some(selected)
            })
            || self.candidates.is_empty() != self.selected_candidate_id.is_none()
        {
            return false;
        }
        let mut canonical = self.clone();
        canonical.graph_sha256.clear();
        self.graph_sha256 == hash_json(&canonical)
    }

    pub fn validate_against(
        &self,
        text: &str,
        active_subject: Option<&str>,
        active_predicates: &[String],
    ) -> bool {
        self.validate_source(text)
            && self.context_sha256 == context_hash(active_subject, active_predicates)
    }

    pub fn validate_source(&self, text: &str) -> bool {
        self.validate() && self.source_text_sha256 == hash_bytes(text.trim().as_bytes())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UtteranceIntentAnalyzer;

impl UtteranceIntentAnalyzer {
    pub fn analyze(
        &self,
        text: &str,
        active_subject: Option<&str>,
        active_predicates: &[String],
    ) -> UtteranceIntentGraphIR {
        analyze_utterance_intent(text, active_subject, active_predicates)
    }
}

fn analyze_utterance_intent(
    text: &str,
    active_subject: Option<&str>,
    active_predicates: &[String],
) -> UtteranceIntentGraphIR {
    let trimmed = text.trim();
    let normalized = trimmed.to_lowercase();
    let matrix_statement =
        crate::grammatical_scope::embedded_information_statement(&normalized).is_some();
    let surface_form = if matrix_statement {
        UtteranceSurfaceFormIR::Declarative
    } else {
        surface_form(&normalized)
    };
    // The embedded content is not an independent problem/explanation demand.
    // Retain the whole observation hash, but do not nominate goals from it.
    let mut signals = if matrix_statement {
        Vec::new()
    } else {
        collect_signals(&normalized)
    };
    signals.truncate(MAX_UTTERANCE_INTENT_SIGNALS);
    let has_context = active_subject.is_some_and(|value| !value.trim().is_empty())
        || !active_predicates.is_empty();
    let mut candidates = infer_candidates(&normalized, active_subject, has_context, &signals);
    candidates.sort_by(|left, right| {
        right
            .score_millis
            .cmp(&left.score_millis)
            .then_with(|| {
                intent_precedence(right.communicative_intent)
                    .cmp(&intent_precedence(left.communicative_intent))
            })
            .then_with(|| left.target.cmp(&right.target))
    });
    candidates.dedup_by(|left, right| {
        left.communicative_intent == right.communicative_intent
            && left.expected_response == right.expected_response
            && left.target == right.target
    });
    candidates.truncate(MAX_UTTERANCE_INTENT_CANDIDATES);
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.candidate_id = format!("UTTERANCE-INTENT-CANDIDATE-{:02}", index + 1);
    }
    let mut unresolved_ambiguities = Vec::new();
    if candidates
        .iter()
        .any(|candidate| candidate.expected_response == ExpectedResponseKindIR::Clarification)
    {
        unresolved_ambiguities.push("PRIOR_DISCOURSE_CONTEXT".to_string());
    }
    let selected_candidate_id = candidates
        .first()
        .map(|candidate| candidate.candidate_id.clone());
    let mut graph = UtteranceIntentGraphIR {
        schema: UTTERANCE_INTENT_GRAPH_SCHEMA.to_string(),
        source_text_sha256: hash_bytes(trimmed.as_bytes()),
        context_sha256: context_hash(active_subject, active_predicates),
        surface_form,
        signals,
        candidates,
        selected_candidate_id,
        unresolved_ambiguities,
        semantic_authority: false,
        external_execution_authorized: false,
        graph_sha256: String::new(),
    };
    graph.graph_sha256 = hash_json(&graph);
    graph
}

fn infer_candidates(
    text: &str,
    active_subject: Option<&str>,
    has_context: bool,
    signals: &[UtteranceIntentSignalIR],
) -> Vec<UtteranceIntentCandidateIR> {
    let has = |kind| signals.iter().any(|signal| signal.kind == kind);
    let signal_ids = |kinds: &[UtteranceSignalKindIR]| {
        signals
            .iter()
            .filter(|signal| kinds.contains(&signal.kind))
            .map(|signal| signal.signal_id.clone())
            .collect::<Vec<_>>()
    };
    let make = |communicative_intent,
                expected_response,
                target: String,
                constraints: Vec<String>,
                evidence_signal_ids,
                score_millis,
                requires_prior_context,
                prior_context_bound| UtteranceIntentCandidateIR {
        candidate_id: String::new(),
        communicative_intent,
        expected_response,
        target,
        constraints,
        evidence_signal_ids,
        score_millis,
        requires_prior_context,
        prior_context_bound,
        semantic_authority: false,
        external_execution_authorized: false,
    };

    let mut candidates = Vec::new();

    if has(UtteranceSignalKindIR::DeliberativeQuestion) {
        candidates.push(make(
            CommunicativeIntentIR::RecommendationRequest,
            ExpectedResponseKindIR::DecisionSupport,
            text.to_string(),
            vec!["identify the missing decision input; do not invent advice or execute".into()],
            signal_ids(&[UtteranceSignalKindIR::DeliberativeQuestion]),
            975,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::ResponseGoalCorrection)
        && has(UtteranceSignalKindIR::ExplanationDemand)
    {
        let target = active_subject
            .filter(|subject| {
                !subject.trim().is_empty() && explanation_target_is_context_bound(text)
            })
            .map(str::to_string)
            .unwrap_or_else(|| explanation_target(text));
        let self_contained = has_explicit_explanation_target(text);
        candidates.push(make(
            CommunicativeIntentIR::ResponseGoalCorrection,
            if has_context || self_contained {
                ExpectedResponseKindIR::Explanation
            } else {
                ExpectedResponseKindIR::Clarification
            },
            target,
            vec![
                "replace the prior response goal; do not retain the superseded action request"
                    .to_string(),
            ],
            signal_ids(&[
                UtteranceSignalKindIR::ResponseGoalCorrection,
                UtteranceSignalKindIR::ExplanationDemand,
            ]),
            980,
            !self_contained,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::ConditionalPremise)
        && has(UtteranceSignalKindIR::ContinuationDecision)
        && has(UtteranceSignalKindIR::BenefitCriterion)
    {
        candidates.push(make(
            CommunicativeIntentIR::ConditionalDecisionRequest,
            ExpectedResponseKindIR::VerifyThenDecide,
            conditional_target(text),
            vec![format!(
                "verify the stated benefit criterion before continuing: {}",
                benefit_constraint(text)
            )],
            signal_ids(&[
                UtteranceSignalKindIR::ConditionalPremise,
                UtteranceSignalKindIR::ContinuationDecision,
                UtteranceSignalKindIR::BenefitCriterion,
            ]),
            970,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::SummaryDemand) {
        let self_contained =
            contains_any(text, &["답변", "설명", "answer", "explanation", "response"]);
        let target = active_subject
            .filter(|subject| !subject.trim().is_empty())
            .map(str::to_string)
            .or_else(|| self_contained.then(|| summary_response_target(text)))
            .unwrap_or_else(|| "prior discourse result".to_string());
        candidates.push(make(
            CommunicativeIntentIR::SummaryRequest,
            if has_context || self_contained {
                ExpectedResponseKindIR::Summary
            } else {
                ExpectedResponseKindIR::Clarification
            },
            target,
            if has_context || self_contained {
                vec!["summarize only claims supported by the prior discourse".to_string()]
            } else {
                vec!["context is required to identify the prior discourse result".to_string()]
            },
            signal_ids(&[UtteranceSignalKindIR::SummaryDemand]),
            if has_context || self_contained {
                950
            } else {
                700
            },
            !self_contained,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::EvidenceDemand) {
        candidates.push(make(
            CommunicativeIntentIR::EvidenceRequest,
            ExpectedResponseKindIR::Evidence,
            evidence_target(text),
            vec!["cite or expose supporting evidence; do not invent support".to_string()],
            signal_ids(&[
                UtteranceSignalKindIR::EvidenceDemand,
                UtteranceSignalKindIR::InterrogativeForm,
            ]),
            960,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::ReadinessOrSafety)
        && has(UtteranceSignalKindIR::InterrogativeForm)
    {
        candidates.push(make(
            CommunicativeIntentIR::AssessmentRequest,
            ExpectedResponseKindIR::Assessment,
            assessment_target(text),
            vec!["assess readiness or safety; do not execute deployment".to_string()],
            signal_ids(&[
                UtteranceSignalKindIR::ReadinessOrSafety,
                UtteranceSignalKindIR::InterrogativeForm,
            ]),
            950,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::AlternativeComparison) {
        let mut constraints = vec!["prefer the safer or better supported alternative".to_string()];
        if has(UtteranceSignalKindIR::PreservationConstraint) {
            constraints.push("preserve the original data and avoid destructive action".to_string());
        }
        candidates.push(make(
            CommunicativeIntentIR::RecommendationRequest,
            ExpectedResponseKindIR::Recommendation,
            recommendation_target(text),
            constraints,
            signal_ids(&[
                UtteranceSignalKindIR::AlternativeComparison,
                UtteranceSignalKindIR::PreservationConstraint,
                UtteranceSignalKindIR::InterrogativeForm,
            ]),
            940,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::ExplanationDemand)
        && has(UtteranceSignalKindIR::InterrogativeForm)
        && !has(UtteranceSignalKindIR::ResponseGoalCorrection)
        && !contains_any(text, &["why don't we", "why dont we", "why not"])
    {
        candidates.push(make(
            CommunicativeIntentIR::ExplanationRequest,
            ExpectedResponseKindIR::Explanation,
            explanation_target(text),
            vec!["explain only from available causal evidence".to_string()],
            signal_ids(&[
                UtteranceSignalKindIR::ExplanationDemand,
                UtteranceSignalKindIR::InterrogativeForm,
            ]),
            920,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::ProblemState)
        && !has(UtteranceSignalKindIR::InterrogativeForm)
        && !has(UtteranceSignalKindIR::ResponseGoalCorrection)
    {
        candidates.push(make(
            CommunicativeIntentIR::ProblemDisclosure,
            ExpectedResponseKindIR::DiagnosisOrNextStep,
            problem_target(text),
            vec![
                "treat the disclosed failure as a request for bounded diagnosis or a next step, not mutation authority"
                    .to_string(),
            ],
            signal_ids(&[UtteranceSignalKindIR::ProblemState]),
            900,
            false,
            has_context,
        ));
    }
    candidates
}

fn intent_precedence(intent: CommunicativeIntentIR) -> u8 {
    match intent {
        CommunicativeIntentIR::ResponseGoalCorrection => 8,
        CommunicativeIntentIR::ConditionalDecisionRequest => 7,
        CommunicativeIntentIR::EvidenceRequest => 6,
        CommunicativeIntentIR::SummaryRequest => 5,
        CommunicativeIntentIR::AssessmentRequest => 4,
        CommunicativeIntentIR::RecommendationRequest => 3,
        CommunicativeIntentIR::ExplanationRequest => 2,
        CommunicativeIntentIR::ProblemDisclosure => 1,
    }
}

fn collect_signals(text: &str) -> Vec<UtteranceIntentSignalIR> {
    let mut signals = Vec::new();
    if decision_inquiry(text).is_some() {
        signals.push(UtteranceIntentSignalIR {
            signal_id: "UTTERANCE-SIGNAL-01".into(),
            kind: UtteranceSignalKindIR::DeliberativeQuestion,
            evidence_surface: text.to_string(),
            byte_start: 0,
            byte_end: text.len(),
            confidence_millis: 975,
        });
    }
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ProblemState,
        &[
            "멈추",
            "멎",
            "깨지",
            "실패",
            "안 돼",
            "안돼",
            "오류",
            "stopping",
            "stops",
            "dies",
            "failed",
            "failure",
            "broken",
            "keeps crashing",
        ],
        900,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ReadinessOrSafety,
        &[
            "배포해도",
            "올려도",
            "되는 상태",
            "괜찮은 거야",
            "ready to",
            "safe to",
            "go live",
            "ship",
            "release safely",
        ],
        940,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::EvidenceDemand,
        &[
            "근거",
            "증거",
            "뒷받침",
            "evidence",
            "supports that",
            "backs up",
            "basis for",
        ],
        960,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::AlternativeComparison,
        &[
            "중 뭐",
            "뭐가 더",
            "어떤 ",
            "제일 나아",
            "which is",
            "which recovery",
            "which approach",
            "best",
            "safer",
            "better option",
        ],
        930,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ResponseGoalCorrection,
        &[
            "아니,",
            "게 아니라",
            "말이 아니야",
            "not asking",
            "wasn't asking",
            "was not asking",
            "i meant",
        ],
        970,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ExplanationDemand,
        &[
            "왜 ",
            "왜 실패",
            "원인",
            "설명",
            "why ",
            "explain",
            "cause",
            "caused",
        ],
        930,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::SummaryDemand,
        &[
            "그래서 결론",
            "핵심만",
            "요지는",
            "bottom line",
            "takeaway",
            "in short",
        ],
        950,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ConditionalPremise,
        &["다면", "라면", "하면", "if ", "provided that"],
        900,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ContinuationDecision,
        &[
            "계속할",
            "진행할",
            "할 만",
            "할만",
            "worth continuing",
            "continue",
            "proceed",
        ],
        920,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::BenefitCriterion,
        &[
            "커버리지",
            "이득",
            "효과",
            "benefit",
            "coverage",
            "payoff",
            "worth",
        ],
        920,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::PreservationConstraint,
        &[
            "잃으면 안",
            "원본 데이터",
            "건드리면 안",
            "cannot lose",
            "must not lose",
            "preserve the original",
            "do not touch the original",
        ],
        970,
    );
    if text.contains('?') {
        signals.push(UtteranceIntentSignalIR {
            signal_id: format!("UTTERANCE-SIGNAL-{:02}", signals.len() + 1),
            kind: UtteranceSignalKindIR::InterrogativeForm,
            evidence_surface: "?".to_string(),
            byte_start: text.rfind('?').unwrap_or_default(),
            byte_end: text.rfind('?').unwrap_or_default() + 1,
            confidence_millis: 1_000,
        });
    }
    signals
}

fn add_signal(
    signals: &mut Vec<UtteranceIntentSignalIR>,
    text: &str,
    kind: UtteranceSignalKindIR,
    markers: &[&str],
    confidence_millis: u16,
) {
    let Some((start, marker)) = markers
        .iter()
        .filter_map(|marker| text.find(marker).map(|start| (start, *marker)))
        .min_by_key(|(start, _)| *start)
    else {
        return;
    };
    signals.push(UtteranceIntentSignalIR {
        signal_id: format!("UTTERANCE-SIGNAL-{:02}", signals.len() + 1),
        kind,
        evidence_surface: marker.to_string(),
        byte_start: start,
        byte_end: start + marker.len(),
        confidence_millis,
    });
}

fn surface_form(text: &str) -> UtteranceSurfaceFormIR {
    if text.contains('?') {
        if text.split_whitespace().count() <= 5 {
            UtteranceSurfaceFormIR::Fragment
        } else {
            UtteranceSurfaceFormIR::Interrogative
        }
    } else if contains_any(
        text,
        &["해줘", "해 줘", "해달라", "please ", "explain ", "inspect "],
    ) {
        UtteranceSurfaceFormIR::Imperative
    } else if text.split_whitespace().count() <= 3 {
        UtteranceSurfaceFormIR::Fragment
    } else {
        UtteranceSurfaceFormIR::Declarative
    }
}

fn problem_target(text: &str) -> String {
    if let Some((prefix, _)) = split_first(text, &["가 계속", "이 계속", "가 또", "이 또"])
    {
        return clean_target(prefix);
    }
    if let Some((prefix, _)) = split_first(
        text,
        &[
            " keeps ",
            " is stopping",
            " stops",
            " dies",
            " failed",
            " keeps crashing",
        ],
    ) {
        return clean_target(prefix);
    }
    clean_target(text)
}

fn assessment_target(text: &str) -> String {
    let without_frame = text
        .strip_prefix("do you think the ")
        .or_else(|| text.strip_prefix("do you think "))
        .unwrap_or(text);
    for marker in [" is actually ready", " is ready", " safe to", " ready to"] {
        if let Some(position) = without_frame.find(marker) {
            let target = clean_target(&without_frame[..position]);
            if !target.is_empty() {
                return target;
            }
        }
    }
    for marker in ["빌드", "운영", "build", "release"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    clean_target(text)
}

fn evidence_target(text: &str) -> String {
    for marker in ["결론", "판단", "recommendation", "conclusion", "claim"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    "stated proposition".to_string()
}

fn recommendation_target(text: &str) -> String {
    for marker in ["복구", "recovery"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    if text.starts_with("which is") || text.starts_with("which option") {
        if let Some((_, tail)) = text.split_once(',') {
            return clean_target(tail);
        }
    }
    if let Some((prefix, _)) = split_first(text, &[" 중 ", " 중", " which", " is safer"]) {
        return clean_target(prefix);
    }
    clean_target(text)
}

fn explanation_target(text: &str) -> String {
    for marker in ["실패", "failed", "failure", "원인", "cause"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    clean_target(text)
}

fn explanation_target_is_context_bound(text: &str) -> bool {
    contains_any(
        text,
        &[
            "why it failed",
            "why that failed",
            "why this failed",
            "why they failed",
            "cause of it",
            "왜 실패했는지",
            "왜 실패한지",
            "왜 실패하는지",
            "왜 실패하는지만",
            "왜 안 됐는지",
            "그게 왜 실패",
            "그것이 왜 실패",
        ],
    )
}

fn summary_response_target(text: &str) -> String {
    for marker in ["답변", "설명", "answer", "explanation", "response"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    "response".to_string()
}

fn has_explicit_explanation_target(text: &str) -> bool {
    contains_any(
        text,
        &[
            "실패 원인",
            "원인만",
            "왜 실패",
            "failure",
            "cause of",
            "why it failed",
            "why it fails",
        ],
    )
}

fn conditional_target(text: &str) -> String {
    for marker in ["통합", "integration"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    split_first(text, &["다면", "라면", "하면", " if "])
        .map(|(prefix, _)| clean_target(prefix))
        .unwrap_or_else(|| clean_target(text))
}

fn benefit_constraint(text: &str) -> String {
    for marker in [
        "커버리지를 넓힌",
        "커버리지",
        "expands coverage",
        "coverage",
        "benefit",
    ] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    "stated benefit".to_string()
}

fn clean_target(value: &str) -> String {
    let cleaned = value
        .trim()
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .trim_start_matches("the ")
        .trim_start_matches("this ")
        .trim_start_matches("이 ")
        .trim();
    if cleaned.is_empty() {
        "utterance subject".to_string()
    } else {
        cleaned.to_string()
    }
}

fn split_first<'a>(text: &'a str, markers: &[&str]) -> Option<(&'a str, &'a str)> {
    markers
        .iter()
        .filter_map(|marker| text.find(marker).map(|start| (start, *marker)))
        .min_by_key(|(start, _)| *start)
        .map(|(start, marker)| (&text[..start], &text[start..start + marker.len()]))
}

fn context_hash(active_subject: Option<&str>, active_predicates: &[String]) -> String {
    let mut predicates = active_predicates.to_vec();
    predicates.sort();
    hash_json(&(
        active_subject.unwrap_or_default().trim().to_lowercase(),
        predicates,
    ))
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

fn hash_json<T: Serialize>(value: &T) -> String {
    hash_bytes(&serde_json::to_vec(value).expect("utterance intent serialization"))
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn decision_source_sha256(source: &str) -> String {
    hash_bytes(source.as_bytes())
}

fn decision_context_sources(inquiry: &DecisionInquiryIR) -> Vec<&str> {
    inquiry
        .inline_context
        .iter()
        .map(|context| context.source_text.as_str())
        .chain(
            inquiry
                .context_evidence
                .iter()
                .map(|context| context.source_text.as_str()),
        )
        .collect()
}

/// Extract a source-local two-to-four-way Korean choice frame.  This is only
/// a bounded grammatical adapter: it does not infer unmentioned properties or
/// bring external knowledge about any candidate into the decision.
fn decision_choice_options(question_source: &str) -> Option<Vec<String>> {
    let question = question_source
        .split(['.', '!', '?', '。', '？'])
        .map(str::trim)
        .find(|unit| unit.contains(" 중"))?;
    let before_middle = question.split_once(" 중")?.0.trim();
    let alternatives = before_middle
        .rsplit_once("에서 ")
        .map(|(_, value)| value)
        .or_else(|| before_middle.rsplit_once("으로 ").map(|(_, value)| value))
        .unwrap_or(before_middle)
        .trim();
    // The conjunctions compose, so normalize each bounded coordination
    // separator before splitting. This keeps a three-way A, B 또는 C question
    // from treating "A, B" as one fabricated alternative.
    let normalized = alternatives
        .replace(" 또는 ", "|")
        .replace("과 ", "|")
        .replace("와 ", "|")
        .replace(',', "|");
    let options = normalized
        .split('|')
        .into_iter()
        .map(str::trim)
        .filter(|option| !option.is_empty() && option.chars().count() <= 128)
        .map(str::to_string)
        .collect::<Vec<_>>();
    ((2..=4).contains(&options.len())
        && options.windows(2).all(|pair| pair[0] != pair[1]))
        .then_some(options)
}

fn choice_feature_tokens(surface: &str) -> std::collections::BTreeSet<String> {
    surface
        .split(|character: char| character.is_whitespace() || matches!(character, ',' | ':' | '(' | ')'))
        .map(|token| token.trim_matches(['.', '!', '?', '？', '。', '"', '\'', '“', '”']))
        .filter(|token| token.chars().count() >= 2 && token.chars().count() <= 32)
        .filter(|token| {
            ["한", "찬", "은", "운", "는", "적인"]
                .iter()
                .any(|ending| token.ends_with(ending))
        })
        .map(str::to_string)
        .collect()
}

fn decision_choice_selection(
    question_source: &str,
    contexts: &[&str],
) -> Option<DecisionChoiceSelectionIR> {
    let options = decision_choice_options(question_source)?;
    let context_features = contexts
        .iter()
        .flat_map(|context| choice_feature_tokens(context))
        .collect::<std::collections::BTreeSet<_>>();
    if context_features.is_empty() {
        return None;
    }
    let scored = options
        .iter()
        .map(|option| {
            let matching_features = choice_feature_tokens(option)
                .intersection(&context_features)
                .cloned()
                .collect::<Vec<_>>();
            (matching_features.len(), matching_features)
        })
        .collect::<Vec<_>>();
    let highest = scored.iter().map(|(score, _)| *score).max()?;
    let selected = scored
        .iter()
        .enumerate()
        .filter(|(_, (score, _))| *score == highest)
        .collect::<Vec<_>>();
    if highest == 0 || selected.len() != 1 {
        return None;
    }
    let (selected_option_index, (_, matching_features)) = selected[0];
    Some(DecisionChoiceSelectionIR {
        question_source: question_source.to_string(),
        question_source_sha256: decision_source_sha256(question_source),
        options: options
            .into_iter()
            .map(|source_text| DecisionChoiceOptionIR {
                source_sha256: decision_source_sha256(&source_text),
                source_text,
            })
            .collect(),
        selected_option_index,
        matching_features: matching_features.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_inquiry_uses_question_role_and_mood_not_task_verbs() {
        for modal in ["should", "could"] {
            for subject in ["I", "we"] {
                let text = format!("What {modal} {subject} do?");
                let inquiry = decision_inquiry(&text).unwrap();
                assert!(inquiry.continues_context && inquiry.validate());
                let mut forged = inquiry;
                forged.continues_context = false;
                assert!(!forged.validate());
            }
        }
        for text in ["What should I read?", "What should I do about entropy?"] {
            assert!(!decision_inquiry(text).unwrap().continues_context);
        }
        for (stem, prospective) in [
            ("먹", "먹을"),
            ("읽", "읽을"),
            ("옮기", "옮길"),
            ("무루하", "무루할"),
        ] {
            for ending in ["까?", "까요?", "지?"] {
                let base = if ending == "지?" { stem } else { prospective };
                let text = format!("그럼 뭘 {base}{ending}");
                let inquiry = decision_inquiry(&text).unwrap();
                assert!(inquiry.validate() && inquiry.continues_context);
                assert_eq!(inquiry.missing_input, DecisionInputIR::Preference);
                let graph = UtteranceIntentAnalyzer.analyze(&text, None, &[]);
                let selected = graph.selected().unwrap();
                assert_eq!(
                    selected.expected_response,
                    ExpectedResponseKindIR::DecisionSupport
                );
                assert_eq!(selected.plan_intent(), None);
                assert!(!selected.external_execution_authorized && !selected.semantic_authority);
            }
        }
        for (text, expected) in [
            ("언제 시작할까?", DecisionInputIR::Deadline),
            ("Then, when should I start?", DecisionInputIR::Deadline),
            ("지금 뭘 하는 게 좋을까?", DecisionInputIR::DesiredOutcome),
            ("What should I do?", DecisionInputIR::DesiredOutcome),
            ("그럼 어떻게 하지?", DecisionInputIR::Constraints),
            ("How should we proceed?", DecisionInputIR::Constraints),
            ("하나만 추천해줘.", DecisionInputIR::Preference),
            ("Please recommend a book.", DecisionInputIR::Preference),
            ("식사 뒤에 카페를 제안해도 될까요?", DecisionInputIR::Preference),
        ] {
            let inquiry = decision_inquiry(text).unwrap();
            assert_eq!(inquiry.missing_input, expected, "{text}");
            for language in [
                crate::language_knowledge::LanguageCodeIR::Korean,
                crate::language_knowledge::LanguageCodeIR::English,
            ] {
                let generated =
                    crate::generative_language::generate_decision_inquiry(language, &inquiry)
                        .unwrap();
                assert!(generated.morphology.realized_text.ends_with('?'));
            }
            let mut tampered = inquiry;
            tampered.source_text = "파일을 삭제해.".into();
            assert!(!tampered.validate());
        }
    }

    #[test]
    fn decision_inquiry_keeps_prior_situation_but_uses_terminal_question() {
        let source = "점심 메뉴를 추천받고 싶어요. 저녁에는 처음 만나는 사람과 식사할 계획인데, 어떤 순서로 준비하면 좋을까요?";
        let inquiry = decision_inquiry(source).expect("terminal advice question");
        assert_eq!(inquiry.source_text, source);
        assert_eq!(inquiry.missing_input, DecisionInputIR::Preference);
        assert!(inquiry.validate());

        let graph = UtteranceIntentAnalyzer.analyze(source, None, &[]);
        assert_eq!(
            graph.selected().map(|candidate| candidate.expected_response),
            Some(ExpectedResponseKindIR::DecisionSupport)
        );
    }

    #[test]
    fn decision_inquiry_retains_postposed_situation_after_one_question() {
        let source = "소개팅 저녁으로 조용한 식당과 활기찬 식당 중 어디가 좋을까요? 상대는 조용한 곳을 선호해요.";
        let inquiry = decision_inquiry(source).expect("one bounded decision unit");
        assert_eq!(inquiry.source_text, source);
        assert_eq!(inquiry.missing_input, DecisionInputIR::Preference);
        assert_eq!(inquiry.inline_context.len(), 1);
        assert_eq!(inquiry.inline_context[0].source_text, "상대는 조용한 곳을 선호해요");
        let selection = inquiry.choice_selection.as_ref().expect("matched choice");
        assert_eq!(
            selection.options[selection.selected_option_index].source_text,
            "조용한 식당"
        );
        assert_eq!(selection.matching_features, vec!["조용한"]);
        assert!(inquiry.validate());
    }

    #[test]
    fn decision_choice_requires_a_literal_candidate_feature_match() {
        let inquiry = decision_inquiry(
            "메밀국수 또는 샤브샤브 중 어디가 좋을까요? 상대는 매운 음식을 못 먹어요.",
        )
        .expect("one bounded decision unit");
        assert_eq!(inquiry.inline_context.len(), 1);
        assert!(inquiry.choice_selection.is_none());
        assert!(inquiry.validate());
    }

    #[test]
    fn decision_choice_supports_active_features_and_three_way_coordination() {
        let active = decision_inquiry(
            "소개팅 저녁으로 조용한 식당과 활기찬 식당 중 어디가 좋을까요? 상대는 활기찬 곳을 선호해요.",
        )
        .expect("decision inquiry");
        let active_selection = active.choice_selection.expect("active match");
        assert_eq!(
            active_selection.options[active_selection.selected_option_index].source_text,
            "활기찬 식당"
        );

        let three_way = decision_inquiry(
            "소개팅 저녁으로 조용한 식당, 활기찬 식당 또는 야외 식당 중 어디가 좋을까요? 상대는 조용한 곳을 선호해요.",
        )
        .expect("decision inquiry");
        let three_way_selection = three_way
            .choice_selection
            .as_ref()
            .expect("three-way match");
        assert_eq!(three_way_selection.options.len(), 3);
        assert_eq!(
            three_way_selection.options[three_way_selection.selected_option_index].source_text,
            "조용한 식당"
        );
        assert!(three_way.validate());
    }

    #[test]
    fn decision_context_evidence_is_source_bound_and_non_question() {
        let inquiry = decision_inquiry("점심 메뉴를 추천해 주세요.").expect("inquiry");
        let context = DecisionInquiryIR::with_context_evidence(
            "매운 음식은 못 먹고 조용한 곳을 좋아해요.",
            &inquiry,
            1,
            2,
        )
        .expect("context");
        assert!(context.validate());
        assert_eq!(context.context_evidence.len(), 1);
        let mut tampered = context;
        tampered.context_evidence[0].source_text = "다른 조건이에요.".into();
        assert!(!tampered.validate());
    }

    #[test]
    fn deictic_preference_question_carries_source_bound_context_forward() {
        let initial = decision_inquiry("점심 메뉴를 추천해 주세요.").expect("inquiry");
        let context = DecisionInquiryIR::with_context_evidence(
            "매운 음식은 못 먹고 조용한 곳을 좋아해요.",
            &initial,
            1,
            2,
        )
        .expect("context");
        let continued = DecisionInquiryIR::continue_with_context(
            "그럼 어디가 좋을까요?",
            &context,
            3,
        )
        .expect("continued question");
        assert!(continued.validate());
        assert!(continued.continues_context);
        assert_eq!(continued.context_evidence, context.context_evidence);
        assert_eq!(continued.source_text, "그럼 어디가 좋을까요?");
    }

    #[test]
    fn decision_inquiry_does_not_consume_reports_negated_requests_or_compound_actions() {
        for past in ["잃어버렸", "썼", "샀", "왔", "봤", "먹었"] {
            for ending in ["지", "죠", "을까", "을까요"] {
                for wh in ["뭘", "언제", "어떻게"] {
                    let text = format!("{wh} {past}{ending}?");
                    assert!(decision_inquiry(&text).is_none(), "{text}");
                }
            }
        }
        for text in [
            "뭘 먹었지?",
            "언제 준비했을까?",
            "어떻게 하는지.",
            "What did I eat?",
            "When is the meeting?",
            "파일을 삭제해.",
            "책을 추천하지 마.",
            "Don't recommend a book.",
            "삭제하고 하나 추천해줘.",
            "Delete the file and recommend a book.",
            "그가 ‘뭘 먹을까?’라고 물었어.",
            "\"What should I do?\"",
            "한가해?",
        ] {
            assert!(decision_inquiry(text).is_none(), "{text}");
        }
    }

    fn selected(text: &str) -> UtteranceIntentCandidateIR {
        UtteranceIntentAnalyzer
            .analyze(text, None, &[])
            .selected()
            .expect("selected utterance intent")
            .clone()
    }

    #[test]
    fn declarative_problem_disclosure_requests_diagnosis_without_execution_authority() {
        let candidate = selected("The upload keeps stopping halfway.");
        assert_eq!(
            candidate.communicative_intent,
            CommunicativeIntentIR::ProblemDisclosure
        );
        assert_eq!(
            candidate.expected_response,
            ExpectedResponseKindIR::DiagnosisOrNextStep
        );
        assert!(!candidate.external_execution_authorized);
    }

    #[test]
    fn deployment_readiness_question_is_assessment_not_deployment() {
        let candidate = selected("Is this build actually ready to deploy?");
        assert_eq!(
            candidate.communicative_intent,
            CommunicativeIntentIR::AssessmentRequest
        );
        assert_eq!(candidate.plan_intent(), Some(PlanIntentIR::Investigate));

        let service = selected(
            "Do you think the Quartz service is actually ready to deploy, or should we check it first?",
        );
        assert_eq!(
            service.communicative_intent,
            CommunicativeIntentIR::AssessmentRequest
        );
        assert_eq!(service.target, "quartz service");
        assert!(!service.external_execution_authorized);
    }

    #[test]
    fn response_goal_correction_replaces_prior_action_with_explanation() {
        let graph = UtteranceIntentAnalyzer.analyze(
            "No, I am not asking you to inspect it; explain why it failed.",
            Some("log"),
            &["INVESTIGATE".to_string()],
        );
        let candidate = graph.selected().expect("correction");
        assert_eq!(
            candidate.communicative_intent,
            CommunicativeIntentIR::ResponseGoalCorrection
        );
        assert_eq!(
            candidate.expected_response,
            ExpectedResponseKindIR::Explanation
        );
        assert!(graph.validate_against(
            "No, I am not asking you to inspect it; explain why it failed.",
            Some("log"),
            &["INVESTIGATE".to_string()]
        ));
    }

    #[test]
    fn self_contained_response_goal_correction_does_not_require_prior_state() {
        let graph = UtteranceIntentAnalyzer.analyze(
            "I was not asking for another run; just explain the cause of the failure.",
            None,
            &[],
        );
        let candidate = graph.selected().expect("self-contained correction");
        assert_eq!(
            candidate.communicative_intent,
            CommunicativeIntentIR::ResponseGoalCorrection
        );
        assert_eq!(
            candidate.expected_response,
            ExpectedResponseKindIR::Explanation
        );
        assert!(!candidate.requires_prior_context);
        assert!(!candidate.prior_context_bound);
        assert!(graph.validate_against(
            "I was not asking for another run; just explain the cause of the failure.",
            None,
            &[]
        ));
    }

    #[test]
    fn advisory_why_dont_we_is_not_reclassified_as_explanation() {
        let graph =
            UtteranceIntentAnalyzer.analyze("Why don't we review the policy first?", None, &[]);
        assert!(graph.selected().is_none());
        assert!(graph.validate_against("Why don't we review the policy first?", None, &[]));
    }

    #[test]
    fn explicit_response_artifact_makes_summary_goal_self_contained() {
        let graph =
            UtteranceIntentAnalyzer.analyze("답변이 너무 길어. 핵심만 다시 설명해", None, &[]);
        let candidate = graph.selected().expect("summary response goal");
        assert_eq!(
            candidate.communicative_intent,
            CommunicativeIntentIR::SummaryRequest
        );
        assert_eq!(candidate.expected_response, ExpectedResponseKindIR::Summary);
        assert!(!candidate.requires_prior_context);
    }

    #[test]
    fn contextless_summary_fails_closed() {
        let graph = UtteranceIntentAnalyzer.analyze("And the takeaway?", None, &[]);
        assert!(graph.requires_clarification());
        assert_eq!(
            graph
                .selected()
                .map(|candidate| candidate.expected_response),
            Some(ExpectedResponseKindIR::Clarification)
        );
    }

    #[test]
    fn compound_response_intents_retain_primary_and_supporting_contributions() {
        let text =
            "If integration expands coverage, should we continue, and what evidence supports that?";
        let graph = UtteranceIntentAnalyzer.analyze(text, None, &[]);
        let active = graph.active().collect::<Vec<_>>();

        assert_eq!(active.len(), 2);
        assert_eq!(
            active[0].communicative_intent,
            CommunicativeIntentIR::ConditionalDecisionRequest
        );
        assert_eq!(
            active[1].communicative_intent,
            CommunicativeIntentIR::EvidenceRequest
        );
        assert_eq!(graph.supporting().count(), 1);
        assert!(graph.validate_against(text, None, &[]));

        let mut tampered = graph;
        tampered.selected_candidate_id = Some("UTTERANCE-INTENT-CANDIDATE-02".to_string());
        assert!(!tampered.validate());
    }

    #[test]
    fn response_goal_correction_subsumes_duplicate_explanation_candidate() {
        let text = "No, I was not asking you to inspect it; explain why it failed.";
        let graph = UtteranceIntentAnalyzer.analyze(text, Some("log"), &[]);

        assert_eq!(graph.active().count(), 1);
        assert_eq!(
            graph
                .selected()
                .map(|candidate| candidate.communicative_intent),
            Some(CommunicativeIntentIR::ResponseGoalCorrection)
        );
        assert!(graph.validate_against(text, Some("log"), &[]));
    }

    #[test]
    fn graph_hash_and_context_binding_reject_tampering() {
        let predicates = vec!["INVESTIGATE".to_string()];
        let graph = UtteranceIntentAnalyzer.analyze(
            "So what is the bottom line?",
            Some("cache state"),
            &predicates,
        );
        assert!(graph.validate_against(
            "So what is the bottom line?",
            Some("cache state"),
            &predicates
        ));
        assert!(!graph.validate_against(
            "So what is the takeaway?",
            Some("cache state"),
            &predicates
        ));
        let mut tampered = graph;
        tampered.candidates[0].target.push_str(" forged");
        assert!(!tampered.validate());
    }
}
