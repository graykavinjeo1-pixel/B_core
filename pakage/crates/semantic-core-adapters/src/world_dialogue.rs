//! Language grounding and episodic premises for the existing core deliberator.
//! No answers, business solutions or language-specific inference rules live here.
//! All linguistic assertions/implications remain conditional on user premises;
//! they are neither verified perceptions nor promoted semantic concepts.

use crate::world_vocabulary::{copular_parts, copular_root, WorldVocabularyIR};
use dockable_semantic_core::{
    ActionAuthorityIR, AuthorityEnvelopeIR, CausalMechanismIR, DeliberationDispositionIR,
    DeliberationEngine, DeliberationIR, DeliberationRequestIR, EvidenceIR, LiteralIR,
    MechanismKindIR, DELIBERATION_REQUEST_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const CAPACITY: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldPropertyIR {
    Active,
    Ready,
    Open,
    Available,
    Safe,
    Valid,
    Connected,
    Powered,
    Registered(String),
}

impl WorldPropertyIR {
    pub const ALL: [Self; 8] = [
        Self::Active,
        Self::Ready,
        Self::Open,
        Self::Available,
        Self::Safe,
        Self::Valid,
        Self::Connected,
        Self::Powered,
    ];
    /// Atomic lexical knowledge, not complete response constructions.
    pub fn expression(&self, korean: bool) -> &'static str {
        match (self, korean) {
            (Self::Active, false) => "active",
            (Self::Active, true) => "가동 상태",
            (Self::Ready, false) => "ready",
            (Self::Ready, true) => "준비 상태",
            (Self::Open, false) => "open",
            (Self::Open, true) => "열림 상태",
            (Self::Available, false) => "available",
            (Self::Available, true) => "사용 가능 상태",
            (Self::Safe, false) => "safe",
            (Self::Safe, true) => "안전 상태",
            (Self::Valid, false) => "valid",
            (Self::Valid, true) => "유효 상태",
            (Self::Connected, false) => "connected",
            (Self::Connected, true) => "연결 상태",
            (Self::Powered, false) => "powered",
            (Self::Powered, true) => "전원 켜짐 상태",
            (Self::Registered(_), _) => "",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WorldAtomIR {
    pub entity: String,
    pub property: WorldPropertyIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
}
impl WorldAtomIR {
    pub fn id(&self) -> String {
        format!("P_{}", hash(self))
    }
    pub fn literal(&self, value: bool) -> LiteralIR {
        LiteralIR {
            proposition_id: self.id(),
            value,
        }
    }
}

// A support claim and a refutation claim are different evidence atoms. This
// dual-rail compilation lets the existing monotone core search both proofs
// without an opposing observation poisoning a potentially valid proof path.
fn encode_support(literal: &LiteralIR) -> LiteralIR {
    LiteralIR {
        proposition_id: format!(
            "{}_{}",
            literal.proposition_id,
            if literal.value { "T" } else { "F" }
        ),
        value: true,
    }
}
pub(crate) fn decode_support(literal: &LiteralIR) -> Option<LiteralIR> {
    if !literal.value {
        return None;
    }
    let (id, polarity) = literal.proposition_id.rsplit_once('_')?;
    Some(LiteralIR {
        proposition_id: id.into(),
        value: match polarity {
            "T" => true,
            "F" => false,
            _ => return None,
        },
    })
}
fn encoded_mechanism(mut mechanism: CausalMechanismIR) -> CausalMechanismIR {
    mechanism.prerequisites = mechanism.prerequisites.iter().map(encode_support).collect();
    mechanism.effects = mechanism.effects.iter().map(encode_support).collect();
    mechanism
}

fn goal_working_set(mut request: DeliberationRequestIR) -> DeliberationRequestIR {
    let mut by_effect = BTreeMap::<&str, Vec<usize>>::new();
    for (index, m) in request.mechanisms.iter().enumerate() {
        for effect in &m.effects {
            by_effect
                .entry(&effect.proposition_id)
                .or_default()
                .push(index);
        }
    }
    let mut needed = request
        .goals
        .iter()
        .map(|g| g.proposition_id.clone())
        .collect::<BTreeSet<_>>();
    let mut frontier = needed.iter().cloned().collect::<Vec<_>>();
    let mut selected = BTreeSet::new();
    while let Some(id) = frontier.pop() {
        for &index in by_effect.get(id.as_str()).into_iter().flatten() {
            if selected.insert(index) {
                for p in &request.mechanisms[index].prerequisites {
                    if needed.insert(p.proposition_id.clone()) {
                        frontier.push(p.proposition_id.clone());
                    }
                }
            }
        }
    }
    request
        .evidence
        .retain(|e| needed.contains(&e.literal.proposition_id));
    request.mechanisms = request
        .mechanisms
        .into_iter()
        .enumerate()
        .filter_map(|(i, m)| selected.contains(&i).then_some(m))
        .collect();
    request
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldPremiseIR {
    pub atom: WorldAtomIR,
    pub value: bool,
    pub introduced_turn: u64,
    pub source_text: String,
    pub source_sha256: String,
    pub active: bool,
    pub answer_binding: Option<WorldAnswerBindingIR>,
    pub lexicon_revision: usize,
    pub grounding_context: WorldDiscourseIR,
    pub reference_resolution: Option<WorldReferenceResolutionIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldAnswerBindingIR {
    pub query: WorldQueryIR,
    pub requested_atom: WorldAtomIR,
    pub decision_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldImplicationIR {
    pub prerequisites: Vec<(WorldAtomIR, bool)>,
    pub effect: (WorldAtomIR, bool),
    pub introduced_turn: u64,
    pub source_text: String,
    pub source_sha256: String,
    pub lexicon_revision: usize,
    pub grounding_context: WorldDiscourseIR,
    pub reference_resolution: Option<WorldReferenceResolutionIR>,
}

/// Discourse reference, not a fact or an execution grant. No response text is
/// cached here. The active proposition supplies roles for subsequent ellipsis.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDiscourseIR {
    pub focus: Option<(WorldAtomIR, bool)>,
    pub source_turn: u64,
}
impl WorldDiscourseIR {
    fn referents(&self) -> Vec<String> {
        self.focus
            .as_ref()
            .map(|(a, _)| {
                let mut ids = vec![a.entity.clone()];
                if let Some(o) = &a.object {
                    if !ids.contains(o) {
                        ids.push(o.clone());
                    }
                }
                ids
            })
            .unwrap_or_default()
    }
    fn validate(&self, turn: u64, vocabulary: &WorldVocabularyIR) -> bool {
        self.source_turn <= turn
            && self
                .focus
                .as_ref()
                .is_none_or(|(a, _)| self.source_turn > 0 && valid_atom(a, vocabulary))
    }
}

/// Source-to-meaning binding for the latest utterance. The previous context is
/// frozen so a later focus change or alias rename cannot reinterpret it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldInputGroundingIR {
    pub source_text: String,
    pub source_sha256: String,
    pub turn: u64,
    pub lexicon_revision: usize,
    pub context: WorldDiscourseIR,
    pub prior_query: Option<WorldQueryIR>,
    pub reference_resolution: Option<WorldReferenceResolutionIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldReferenceGapIR {
    pub source_text: String,
    pub source_sha256: String,
    pub turn: u64,
    pub lexicon_revision: usize,
    pub context: WorldDiscourseIR,
    pub prior_query: Option<WorldQueryIR>,
}
impl WorldReferenceGapIR {
    pub fn candidates(&self) -> Vec<String> {
        self.context.referents()
    }
    fn validate(&self, vocabulary: &WorldVocabularyIR) -> bool {
        if self.turn == 0
            || self.source_sha256 != hash(&self.source_text)
            || !self.context.validate(self.turn - 1, vocabulary)
        {
            return false;
        }
        let candidates = self.candidates();
        if candidates.len() != 2
            || parse_input_with_vocabulary(
                &self.source_text,
                self.prior_query.as_ref(),
                vocabulary,
                self.lexicon_revision,
                &self.context,
                None,
            )
            .is_some()
        {
            return false;
        }
        let meanings = candidates
            .iter()
            .map(|c| {
                parse_input_with_vocabulary(
                    &self.source_text,
                    self.prior_query.as_ref(),
                    vocabulary,
                    self.lexicon_revision,
                    &self.context,
                    Some(c),
                )
            })
            .collect::<Option<Vec<_>>>();
        meanings.is_some_and(|m| m[0] != m[1])
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldReferenceResolutionIR {
    pub gap: WorldReferenceGapIR,
    pub selected: String,
}

fn selected_reference(text: &str, gap: &WorldReferenceGapIR) -> Option<String> {
    let lower = text.trim().trim_end_matches(['.', '!']).to_lowercase();
    gap.candidates().into_iter().find(|c| {
        lower == *c
            || lower == format!("{c}야")
            || lower == format!("{c}이야")
            || (c == "__user__" && matches!(lower.as_str(), "me" | "나" | "저" | "나야"))
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldClarificationIR {
    pub memory: DialogueWorldIR,
    pub gap: WorldReferenceGapIR,
}
impl WorldClarificationIR {
    pub fn validate(&self) -> bool {
        self.memory.validate(self.memory.latest_turn())
            && self.gap.validate(&self.memory.vocabulary)
            && self.memory.pending_reference.as_ref() == Some(&self.gap)
    }
    pub fn into_answer(
        self,
        language: crate::language_knowledge::LanguageCodeIR,
    ) -> Result<crate::discourse_qa::DiscourseAnswerIR, String> {
        let mut answer =
            crate::discourse_qa::DiscourseQaEngine.unanswered(&self.gap.source_text, language);
        answer.claims.clear();
        answer.disposition = crate::discourse_qa::DiscourseAnswerDispositionIR::AmbiguousQuery;
        answer.realized_text =
            crate::generative_language::generate_world_clarification(language, &self)?
                .morphology
                .realized_text;
        answer.world_clarification = Some(self);
        Ok(answer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldQueryIR {
    pub target: (WorldAtomIR, bool),
    pub explain: bool,
    /// A local intervention is never written into factual memory.
    pub assumption: Option<(WorldAtomIR, bool)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueWorldIR {
    pub vocabulary: WorldVocabularyIR,
    pub premises: Vec<WorldPremiseIR>,
    pub implications: Vec<WorldImplicationIR>,
    pub last_query: Option<WorldQueryIR>,
    pub discourse: WorldDiscourseIR,
    pub last_grounding: Option<WorldInputGroundingIR>,
    pub pending_reference: Option<WorldReferenceGapIR>,
}
impl Default for DialogueWorldIR {
    fn default() -> Self {
        Self {
            vocabulary: WorldVocabularyIR::conversational(),
            premises: vec![],
            implications: vec![],
            last_query: None,
            discourse: WorldDiscourseIR::default(),
            last_grounding: None,
            pending_reference: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedInput {
    Premise((WorldAtomIR, bool), bool),
    Implication(Vec<(WorldAtomIR, bool)>, (WorldAtomIR, bool)),
    Query(WorldQueryIR),
}

pub struct PreparedWorldTurn {
    pub memory: DialogueWorldIR,
    pub query: Option<WorldQueryIR>,
    pub recognized: bool,
    pub clarification: Option<WorldClarificationIR>,
}

/// Acknowledgement of a typed memory update, not an inferred world fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMemoryUpdateIR {
    pub memory: DialogueWorldIR,
    pub turn: u64,
    pub source_text: String,
}
impl WorldMemoryUpdateIR {
    pub fn validate(&self) -> bool {
        self.memory.validate(self.turn)
            && match self.memory.parse_at(
                &self.source_text,
                None,
                self.memory.vocabulary.revision(),
                &self
                    .memory
                    .last_grounding
                    .as_ref()
                    .map(|g| g.context.clone())
                    .unwrap_or_default(),
                self.memory
                    .last_grounding
                    .as_ref()
                    .and_then(|g| g.reference_resolution.as_ref()),
            ) {
                Some(ParsedInput::Premise((atom, value), _)) => {
                    self.memory.premises.last().is_some_and(|p| {
                        p.introduced_turn == self.turn
                            && p.active
                            && p.atom == atom
                            && p.value == value
                            && p.source_text == self.source_text
                    })
                }
                Some(ParsedInput::Implication(prerequisites, effect)) => {
                    self.memory.implications.last().is_some_and(|r| {
                        r.introduced_turn == self.turn
                            && r.prerequisites == prerequisites
                            && r.effect == effect
                            && r.source_text == self.source_text
                    })
                }
                _ => false,
            }
    }
    pub fn into_answer(
        self,
        language: crate::language_knowledge::LanguageCodeIR,
    ) -> Result<crate::discourse_qa::DiscourseAnswerIR, String> {
        let mut answer =
            crate::discourse_qa::DiscourseQaEngine.unanswered(&self.source_text, language);
        answer.claims.clear();
        answer.disposition =
            crate::discourse_qa::DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords;
        answer.realized_text =
            crate::generative_language::generate_world_memory_update(language, &self)?
                .morphology
                .realized_text;
        answer.world_memory_update = Some(self);
        Ok(answer)
    }
}

impl DialogueWorldIR {
    fn parse_at(
        &self,
        text: &str,
        last: Option<&WorldQueryIR>,
        revision: usize,
        context: &WorldDiscourseIR,
        resolution: Option<&WorldReferenceResolutionIR>,
    ) -> Option<ParsedInput> {
        if let Some(r) = resolution {
            if !r.gap.validate(&self.vocabulary)
                || selected_reference(text, &r.gap).as_ref() != Some(&r.selected)
            {
                return None;
            }
            return parse_input_with_vocabulary(
                &r.gap.source_text,
                r.gap.prior_query.as_ref(),
                &self.vocabulary,
                r.gap.lexicon_revision,
                &r.gap.context,
                Some(&r.selected),
            );
        }
        parse_input_with_vocabulary(text, last, &self.vocabulary, revision, context, None)
    }

    pub fn latest_turn(&self) -> u64 {
        self.premises
            .iter()
            .map(|p| p.introduced_turn)
            .chain(self.implications.iter().map(|r| r.introduced_turn))
            .chain([self.discourse.source_turn])
            .chain(self.last_grounding.iter().map(|g| g.turn))
            .chain(self.pending_reference.iter().map(|g| g.turn))
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn clear_discourse(&mut self) {
        self.last_query = None;
        self.discourse = WorldDiscourseIR::default();
        self.last_grounding = None;
        self.pending_reference = None;
    }

    pub fn accepts_observation_reply(&self, text: &str) -> bool {
        self.pending_reference
            .as_ref()
            .is_some_and(|g| selected_reference(text, g).is_some())
            || (self
                .last_query
                .as_ref()
                .is_some_and(|q| q.assumption.is_none())
                && boolean_reply(text).is_some())
    }

    pub fn prepare(&self, text: &str, turn: u64) -> Result<PreparedWorldTurn, String> {
        if !self.validate(turn.saturating_sub(1)) {
            return Err("INVALID_WORLD_MEMORY".into());
        }
        let mut memory = self.clone();
        let resolution = self.pending_reference.as_ref().and_then(|gap| {
            selected_reference(text, gap).map(|selected| WorldReferenceResolutionIR {
                gap: gap.clone(),
                selected,
            })
        });
        let mut parsed = self.parse_at(
            text,
            self.last_query.as_ref(),
            self.vocabulary.revision(),
            &self.discourse,
            resolution.as_ref(),
        );
        if parsed.is_none() && resolution.is_none() {
            let gap = WorldReferenceGapIR {
                source_text: text.into(),
                source_sha256: hash(&text),
                turn,
                lexicon_revision: self.vocabulary.revision(),
                context: self.discourse.clone(),
                prior_query: self.last_query.clone(),
            };
            if gap.validate(&self.vocabulary) {
                memory.pending_reference = Some(gap.clone());
                memory.last_grounding = None;
                return Ok(PreparedWorldTurn {
                    clarification: Some(WorldClarificationIR {
                        memory: memory.clone(),
                        gap,
                    }),
                    memory,
                    query: None,
                    recognized: true,
                });
            }
        }
        memory.pending_reference = None;
        let mut answer_binding = None;
        if let Some(prior_query) = self.last_query.as_ref().filter(|q| q.assumption.is_none()) {
            let previous = deliberate_world(self, prior_query)?;
            if let Some(question) = previous.decision.question.as_ref() {
                let atom = previous
                    .atoms
                    .get(&question.proposition_id)
                    .ok_or("MISSING_QUESTION_ATOM")?
                    .clone();
                let reply = boolean_reply(text).or_else(|| match &parsed {
                    Some(ParsedInput::Premise((a, value), _)) if a == &atom => Some(*value),
                    _ => None,
                });
                if let Some(value) = reply {
                    let correction = matches!(parsed, Some(ParsedInput::Premise(_, true)));
                    parsed = Some(ParsedInput::Premise((atom.clone(), value), correction));
                    answer_binding = Some(WorldAnswerBindingIR {
                        query: prior_query.clone(),
                        requested_atom: atom,
                        decision_sha256: previous.semantic_decision_sha256,
                    });
                }
            }
        }
        let recognized = parsed.is_some();
        memory.last_grounding = recognized.then(|| WorldInputGroundingIR {
            source_text: text.into(),
            source_sha256: hash(&text),
            turn,
            lexicon_revision: self.vocabulary.revision(),
            context: self.discourse.clone(),
            prior_query: self.last_query.clone(),
            reference_resolution: resolution.clone(),
        });
        let mut query = None;
        match parsed {
            Some(ParsedInput::Premise((atom, value), correction)) => {
                memory.discourse = WorldDiscourseIR {
                    focus: Some((atom.clone(), value)),
                    source_turn: turn,
                };
                if memory.premises.len() == CAPACITY {
                    return Err("WORLD_PREMISE_CAPACITY".into());
                }
                if correction {
                    for prior in &mut memory.premises {
                        if prior.atom == atom {
                            prior.active = false;
                        }
                    }
                }
                memory.premises.push(WorldPremiseIR {
                    atom,
                    value,
                    introduced_turn: turn,
                    source_text: text.into(),
                    source_sha256: hash(&text),
                    active: true,
                    answer_binding: answer_binding.clone(),
                    lexicon_revision: self.vocabulary.revision(),
                    grounding_context: self.discourse.clone(),
                    reference_resolution: resolution.clone(),
                });
                memory.last_query = answer_binding.as_ref().map(|b| b.query.clone());
                query = memory.last_query.clone();
            }
            Some(ParsedInput::Implication(prerequisites, effect)) => {
                memory.discourse = WorldDiscourseIR {
                    focus: Some(effect.clone()),
                    source_turn: turn,
                };
                if memory.implications.len() == CAPACITY {
                    return Err("WORLD_RULE_CAPACITY".into());
                }
                memory.implications.push(WorldImplicationIR {
                    prerequisites,
                    effect,
                    introduced_turn: turn,
                    source_text: text.into(),
                    source_sha256: hash(&text),
                    lexicon_revision: self.vocabulary.revision(),
                    grounding_context: self.discourse.clone(),
                    reference_resolution: resolution.clone(),
                });
                memory.last_query = None;
            }
            Some(ParsedInput::Query(q)) => {
                query = Some(q.clone());
                memory.last_query = Some(q);
            }
            None => {
                // A new unsupported topic must not inherit a world question.
                if !matches!(
                    text.trim()
                        .trim_end_matches(['.', '!'])
                        .to_lowercase()
                        .as_str(),
                    "thanks" | "고마워" | "감사합니다"
                ) {
                    memory.clear_discourse();
                }
            }
        }
        if let Some(q) = &query {
            // The next utterance is about the proposition actually asked by the
            // core, not necessarily the original goal. Explicit statements as
            // well as yes/no replies can fill that information slot.
            let reasoning = deliberate_world_inner(&memory, q)?;
            let focus = reasoning
                .decision
                .question
                .as_ref()
                .and_then(|lit| {
                    reasoning
                        .atoms
                        .get(&lit.proposition_id)
                        .map(|a| (a.clone(), lit.value))
                })
                .unwrap_or_else(|| q.target.clone());
            memory.discourse = WorldDiscourseIR {
                focus: Some(focus),
                source_turn: turn,
            };
        }
        Ok(PreparedWorldTurn {
            memory,
            query,
            recognized,
            clarification: None,
        })
    }

    pub fn validate(&self, turn: u64) -> bool {
        self.vocabulary.validate() && self.premises.len() <= CAPACITY && self.implications.len() <= CAPACITY
            && self.premises.windows(2).all(|pair| pair[0].introduced_turn < pair[1].introduced_turn)
            && self.implications.windows(2).all(|pair| pair[0].introduced_turn < pair[1].introduced_turn)
            && self.premises.iter().all(|p| p.active != self.premises.iter().any(|later|
                later.introduced_turn > p.introduced_turn && later.atom == p.atom
                && matches!(self.parse_at(&later.source_text, None, later.lexicon_revision, &later.grounding_context, later.reference_resolution.as_ref()), Some(ParsedInput::Premise(_, true)))))
            && self.premises.iter().all(|p| p.introduced_turn > 0 && p.introduced_turn <= turn
                && p.source_sha256 == hash(&p.source_text)
                && p.lexicon_revision <= self.vocabulary.revision()
                && p.grounding_context.validate(p.introduced_turn.saturating_sub(1), &self.vocabulary)
                && if let Some(binding) = &p.answer_binding {
                    (boolean_reply(&p.source_text) == Some(p.value)
                        || matches!(self.parse_at(&p.source_text, None, p.lexicon_revision, &p.grounding_context, p.reference_resolution.as_ref()),Some(ParsedInput::Premise((a,v),_)) if a == p.atom && v == p.value))
                        && binding.requested_atom == p.atom
                        && valid_atom(&p.atom, &self.vocabulary) && valid_atom(&binding.query.target.0, &self.vocabulary)
                        && binding.query.assumption.is_none()
                } else {matches!(self.parse_at(&p.source_text, None, p.lexicon_revision, &p.grounding_context, p.reference_resolution.as_ref()), Some(ParsedInput::Premise((a,v), _)) if a == p.atom && v == p.value)})
            && self.implications.iter().all(|r| r.introduced_turn > 0 && r.introduced_turn <= turn
                && r.source_sha256 == hash(&r.source_text)
                && r.grounding_context.validate(r.introduced_turn.saturating_sub(1), &self.vocabulary)
                && matches!(self.parse_at(&r.source_text, None, r.lexicon_revision, &r.grounding_context, r.reference_resolution.as_ref()), Some(ParsedInput::Implication(p,e)) if p == r.prerequisites && e == r.effect))
            && self.last_query.as_ref().is_none_or(|q| valid_atom(&q.target.0, &self.vocabulary)
                && q.assumption.as_ref().is_none_or(|(a,_)| valid_atom(a, &self.vocabulary)))
            && self.answer_bindings_valid()
            && self.discourse.validate(turn, &self.vocabulary)
            && self.pending_reference.as_ref().is_none_or(|g| g.turn<=turn && g.validate(&self.vocabulary))
            && self.last_grounding.as_ref().is_none_or(|g| g.turn > 0 && g.turn <= turn
                && g.source_sha256 == hash(&g.source_text)
                && g.context.validate(g.turn-1,&self.vocabulary)
                && g.lexicon_revision <= self.vocabulary.revision()
                && self.grounding_matches_memory(g))
    }

    fn grounding_matches_memory(&self, g: &WorldInputGroundingIR) -> bool {
        match self.parse_at(
            &g.source_text,
            g.prior_query.as_ref(),
            g.lexicon_revision,
            &g.context,
            g.reference_resolution.as_ref(),
        ) {
            Some(ParsedInput::Query(q)) => self.last_query.as_ref() == Some(&q),
            Some(ParsedInput::Premise((a, v), _)) => self.premises.iter().any(|p| {
                p.introduced_turn == g.turn
                    && p.source_text == g.source_text
                    && p.atom == a
                    && p.value == v
                    && p.grounding_context == g.context
            }),
            Some(ParsedInput::Implication(p, e)) => self.implications.iter().any(|r| {
                r.introduced_turn == g.turn
                    && r.source_text == g.source_text
                    && r.prerequisites == p
                    && r.effect == e
                    && r.grounding_context == g.context
            }),
            None => self.premises.iter().any(|p| {
                p.introduced_turn == g.turn
                    && p.source_text == g.source_text
                    && p.answer_binding.is_some()
                    && boolean_reply(&g.source_text) == Some(p.value)
            }),
        }
    }

    fn answer_bindings_valid(&self) -> bool {
        // Check each reply against the question the core would actually have
        // asked from the preceding memory. Use the already shape-checked inner
        // deliberator, avoiding recursive validation of the episode history.
        self.premises
            .iter()
            .filter_map(|p| p.answer_binding.as_ref().map(|b| (p, b)))
            .all(|(p, b)| {
                let mut prefix = self.clone();
                prefix
                    .premises
                    .retain(|earlier| earlier.introduced_turn < p.introduced_turn);
                prefix
                    .implications
                    .retain(|earlier| earlier.introduced_turn < p.introduced_turn);
                let corrections = prefix
                    .premises
                    .iter()
                    .filter(|earlier| {
                        matches!(
                            self.parse_at(
                                &earlier.source_text,
                                None,
                                earlier.lexicon_revision,
                                &earlier.grounding_context,
                                earlier.reference_resolution.as_ref(),
                            ),
                            Some(ParsedInput::Premise(_, true))
                        )
                    })
                    .map(|earlier| (earlier.atom.clone(), earlier.introduced_turn))
                    .collect::<Vec<_>>();
                for earlier in &mut prefix.premises {
                    earlier.active = !corrections.iter().any(|(atom, turn)| {
                        atom == &earlier.atom && *turn > earlier.introduced_turn
                    });
                }
                prefix.last_query = Some(b.query.clone());
                deliberate_world_inner(&prefix, &b.query).is_ok_and(|result| {
                    result.semantic_decision_sha256 == b.decision_sha256
                        && result
                            .decision
                            .question
                            .as_ref()
                            .is_some_and(|q| q.proposition_id == b.requested_atom.id())
                })
            })
    }
}

fn boolean_reply(text: &str) -> Option<bool> {
    match text
        .trim()
        .trim_end_matches(['.', '!'])
        .to_lowercase()
        .as_str()
    {
        "yes" | "응" | "네" | "맞아" => Some(true),
        "no" | "아니" | "아니야" => Some(false),
        _ => None,
    }
}

fn valid_atom(atom: &WorldAtomIR, vocabulary: &WorldVocabularyIR) -> bool {
    vocabulary.accepts_atom(atom)
        && valid_entity(&atom.entity)
        && atom.object.as_ref().is_none_or(|o| valid_entity(o))
}

fn valid_entity(entity: &str) -> bool {
    !entity.is_empty()
        && entity.chars().count() <= 48
        && entity == entity.to_lowercase()
        && !matches!(
            entity,
            "it" | "this"
                | "that"
                | "he"
                | "she"
                | "they"
                | "we"
                | "you"
                | "i"
                | "그것"
                | "그거"
                | "이것"
                | "저것"
                | "그"
                | "나"
                | "너"
        )
        && entity
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
}

fn parse_atom_with_vocabulary(
    text: &str,
    vocabulary: &WorldVocabularyIR,
    revision: usize,
) -> Option<(WorldAtomIR, bool)> {
    let lower = text
        .trim()
        .trim_end_matches(['.', '?', '!'])
        .trim()
        .to_lowercase();
    vocabulary.lexical_history.get(revision)?;
    if let Some((atom, value)) = vocabulary.parse(&lower, revision) {
        return Some((atom, value));
    }
    let (entity, predicate) = copular_parts(&lower)?;
    let (predicate, value) = copular_root(predicate);
    let property = WorldPropertyIR::ALL
        .into_iter()
        .find(|p| predicate == p.expression(false) || predicate == p.expression(true))?;
    let atom = WorldAtomIR {
        entity: entity.into(),
        property,
        object: None,
    };
    Some((atom, value))
}

fn parse_contextual_atom(
    text: &str,
    vocabulary: &WorldVocabularyIR,
    revision: usize,
    reference: Option<&str>,
) -> Option<(WorldAtomIR, bool)> {
    let parse = |s: &str| parse_atom_with_vocabulary(s, vocabulary, revision);
    let (mut atom, value) = parse(text)
        .or_else(|| reference.and_then(|_| parse(&format!("it is {text}"))))
        .or_else(|| reference.and_then(|_| parse(&format!("그것은 {text}"))))?;
    for entity in std::iter::once(&mut atom.entity).chain(atom.object.iter_mut()) {
        if matches!(entity.as_str(), "i" | "me" | "나" | "저") {
            *entity = "__user__".into();
        } else if matches!(
            entity.as_str(),
            "it" | "that" | "this" | "그것" | "그거" | "이것"
        ) {
            *entity = reference?.into();
        }
    }
    valid_atom(&atom, vocabulary).then_some((atom, value))
}

fn parse_input_with_vocabulary(
    text: &str,
    last: Option<&WorldQueryIR>,
    vocabulary: &WorldVocabularyIR,
    revision: usize,
    context: &WorldDiscourseIR,
    forced_reference: Option<&str>,
) -> Option<ParsedInput> {
    let referents = context.referents();
    let reference =
        forced_reference.or_else(|| (referents.len() == 1).then(|| referents[0].as_str()));
    let parse_atom = |text: &str| parse_contextual_atom(text, vocabulary, revision, reference);
    if text.chars().count() > 2048 || text.contains(['"', '“', '”', '‘', '’', '`']) {
        return None;
    }
    let lower = text.trim().to_lowercase();
    let mut normalized = lower.as_str();
    // Discourse particles carry floor/transition information, not world facts.
    for _ in 0..3 {
        if let Some(rest) = ["음, ", "어, ", "um, ", "well, ", "그럼, ", "then, "]
            .iter()
            .find_map(|p| normalized.strip_prefix(p))
        {
            normalized = rest;
        } else {
            break;
        }
    }
    let lower = normalized;
    let text = lower.trim_end_matches(['.', '?', '!']).trim();
    let why = matches!(text, "why" | "왜" | "왜 그래" | "그 이유는");
    if why || matches!(text, "so" | "then" | "그럼" | "그러면") {
        return last
            .cloned()
            .or_else(|| {
                context.focus.clone().map(|target| WorldQueryIR {
                    target,
                    explain: false,
                    assumption: None,
                })
            })
            .map(|mut q| {
                q.explain = why;
                ParsedInput::Query(q)
            });
    }
    if lower.ends_with('?') {
        let contrasted = text
            .strip_prefix("what about ")
            .or_else(|| text.strip_prefix("and "))
            .or_else(|| text.strip_suffix(['은', '는']).filter(|s| valid_entity(s)));
        if let (Some(entity), Some((focus, value))) = (contrasted, &context.focus) {
            if !valid_entity(entity) || focus.object.as_deref() == Some(entity) {
                return None;
            }
            let mut atom = focus.clone();
            atom.entity = entity.into();
            return Some(ParsedInput::Query(WorldQueryIR {
                target: (atom, *value),
                explain: false,
                assumption: None,
            }));
        }
    }
    if let Some(body) = text
        .strip_prefix("suppose ")
        .or_else(|| text.strip_prefix("가정: "))
    {
        let (assumption, question) = body.split_once(". ")?;
        if !lower.ends_with('?') {
            return None;
        }
        let target = parse_atom(question)?;
        return Some(ParsedInput::Query(WorldQueryIR {
            target,
            explain: false,
            assumption: Some(parse_atom(assumption)?),
        }));
    }
    let conditional = text
        .strip_prefix("if ")
        .and_then(|body| body.split_once(", then "))
        .or_else(|| text.split_once("이면 "));
    // Keep verbal endings in the antecedent so the lexical grammar, not a
    // whole-sentence dispatcher, resolves 하다 and negative conditional forms.
    let conditional = conditional.or_else(|| {
        ["하면 ", "않으면 "].iter().find_map(|ending| {
            text.find(ending)
                .map(|i| (&text[..i + ending.len() - 1], &text[i + ending.len()..]))
        })
    });
    if let Some((lhs, rhs)) = conditional {
        if lower.ends_with('?') {
            return None;
        }
        let parts = if lhs.contains(" and ") {
            lhs.split(" and ").collect::<Vec<_>>()
        } else if lhs.contains("이고 ") {
            lhs.split("이고 ").collect::<Vec<_>>()
        } else {
            lhs.split(" 그리고 ").collect::<Vec<_>>()
        };
        if parts.len() > 4 {
            return None;
        }
        let prerequisites = parts
            .into_iter()
            .map(parse_atom)
            .collect::<Option<Vec<_>>>()?;
        return Some(ParsedInput::Implication(prerequisites, parse_atom(rhs)?));
    }
    let (body, why) = text
        .strip_prefix("why ")
        .or_else(|| text.strip_prefix("왜 "))
        .map_or((text, false), |body| (body, true));
    if lower.ends_with('?') || why {
        return parse_atom(body).map(|target| {
            ParsedInput::Query(WorldQueryIR {
                target,
                explain: why,
                assumption: None,
            })
        });
    }
    let (body, correction) = text
        .strip_prefix("actually, ")
        .or_else(|| text.strip_prefix("정정: "))
        .or_else(|| text.strip_prefix("아니, "))
        .or_else(|| text.strip_prefix("no, "))
        .map_or((text, false), |body| (body, true));
    parse_atom(body).map(|a| ParsedInput::Premise(a, correction))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldVerdictIR {
    Supported,
    Refuted,
    Conflict,
    Unknown,
    ResourceBound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDecisionIR {
    pub verdict: WorldVerdictIR,
    pub target: LiteralIR,
    pub conclusion: Option<LiteralIR>,
    pub question: Option<LiteralIR>,
    pub proof_mechanism_ids: Vec<String>,
    pub premise_evidence_ids: Vec<String>,
    pub hypothetical: bool,
    pub external_action_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldMovePurposeIR {
    CauseUnknown,
    Premise,
    Hypothesis,
    Inference,
    Conclusion,
    Conflict,
    Unknown,
    Bound,
    Ask,
}
impl WorldMovePurposeIR {
    pub(crate) fn mode(self) -> &'static str {
        match self {
            Self::CauseUnknown => "CAUSE_UNKNOWN",
            Self::Premise => "PREMISE",
            Self::Hypothesis => "HYPOTHESIS",
            Self::Inference => "DERIVED",
            Self::Conclusion => "CONCLUSION",
            Self::Conflict => "CONFLICT",
            Self::Unknown => "UNKNOWN",
            Self::Bound => "BOUND",
            Self::Ask => "ASK",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldUtteranceMoveIR {
    pub proposition: LiteralIR,
    pub purpose: WorldMovePurposeIR,
    pub evidence_refs: Vec<String>,
}

/// Decide what to communicate before choosing words. This plan carries no
/// sentence templates, and is replayed alongside the underlying core decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldUtterancePlanIR {
    pub decision_sha256: String,
    pub moves: Vec<WorldUtteranceMoveIR>,
    pub explains: bool,
}

fn plan_utterance(
    query: &WorldQueryIR,
    decision: &WorldDecisionIR,
    requests: &[DeliberationRequestIR],
) -> Result<WorldUtterancePlanIR, String> {
    use WorldMovePurposeIR as P;
    let mut moves: Vec<WorldUtteranceMoveIR> = Vec::new();
    if query.explain && decision.proof_mechanism_ids.is_empty() && !decision.hypothetical {
        if let Some(proposition) = &decision.conclusion {
            return Ok(WorldUtterancePlanIR {
                decision_sha256: hash(decision),
                explains: true,
                moves: vec![WorldUtteranceMoveIR {
                    proposition: proposition.clone(),
                    purpose: P::CauseUnknown,
                    evidence_refs: decision.premise_evidence_ids.clone(),
                }],
            });
        }
    }
    if query.explain && decision.conclusion.is_some() {
        let request = &requests[usize::from(decision.verdict == WorldVerdictIR::Refuted)];
        let mut dependencies = BTreeSet::from([decision.target.proposition_id.clone()]);
        for id in decision.proof_mechanism_ids.iter().rev() {
            let rule = request
                .mechanisms
                .iter()
                .find(|r| &r.mechanism_id == id)
                .ok_or("MISSING_PROOF_STEP")?;
            dependencies.extend(
                rule.prerequisites
                    .iter()
                    .filter_map(decode_support)
                    .map(|l| l.proposition_id),
            );
        }
        for e in &request.evidence {
            let proposition = decode_support(&e.literal).ok_or("INVALID_PROOF_EVIDENCE")?;
            if dependencies.contains(&proposition.proposition_id) {
                moves.push(WorldUtteranceMoveIR {
                    proposition,
                    purpose: if e.evidence_id == "HYPOTHETICAL" {
                        P::Hypothesis
                    } else {
                        P::Premise
                    },
                    evidence_refs: vec![e.evidence_id.clone()],
                });
            }
        }
        for id in &decision.proof_mechanism_ids {
            let rule = request
                .mechanisms
                .iter()
                .find(|r| &r.mechanism_id == id)
                .ok_or("MISSING_PROOF_STEP")?;
            for effect in &rule.effects {
                moves.push(WorldUtteranceMoveIR {
                    proposition: decode_support(effect).ok_or("INVALID_PROOF_EFFECT")?,
                    purpose: P::Inference,
                    evidence_refs: vec![id.clone()],
                });
            }
        }
    }
    let proposition = decision
        .conclusion
        .clone()
        .unwrap_or_else(|| decision.target.clone());
    // The last proof effect already says the conclusion. Preserve its evidence
    // while avoiding an extra sentence repeating exactly that proposition.
    if moves.last().is_none_or(|m| m.proposition != proposition) {
        moves.push(WorldUtteranceMoveIR {
            proposition,
            purpose: match decision.verdict {
                WorldVerdictIR::Supported | WorldVerdictIR::Refuted => {
                    if decision.hypothetical {
                        P::Hypothesis
                    } else {
                        P::Conclusion
                    }
                }
                WorldVerdictIR::Conflict => P::Conflict,
                WorldVerdictIR::Unknown => P::Unknown,
                WorldVerdictIR::ResourceBound => P::Bound,
            },
            evidence_refs: vec![format!("DECISION:{:?}", decision.verdict)],
        });
    }
    if let Some(question) = &decision.question {
        moves.push(WorldUtteranceMoveIR {
            proposition: question.clone(),
            purpose: P::Ask,
            evidence_refs: vec!["MISSING_PREMISE".into()],
        });
    }
    Ok(WorldUtterancePlanIR {
        decision_sha256: hash(decision),
        moves,
        explains: query.explain,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldReasoningIR {
    pub memory: DialogueWorldIR,
    pub query: WorldQueryIR,
    pub requests: Vec<DeliberationRequestIR>,
    pub deliberations: Vec<DeliberationIR>,
    pub atoms: BTreeMap<String, WorldAtomIR>,
    pub decision: WorldDecisionIR,
    pub semantic_decision_sha256: String,
    pub utterance_plan: WorldUtterancePlanIR,
}

impl WorldReasoningIR {
    pub fn matches_question(&self, text: &str) -> bool {
        let grounding = self
            .memory
            .last_grounding
            .as_ref()
            .filter(|g| g.source_text == text);
        let context = grounding
            .map(|g| g.context.clone())
            .unwrap_or_else(|| self.memory.discourse.clone());
        let prior_query = grounding
            .and_then(|g| g.prior_query.as_ref())
            .or(self.memory.last_query.as_ref());
        let revision = grounding
            .map(|g| g.lexicon_revision)
            .unwrap_or(self.memory.vocabulary.revision());
        matches!(self.memory.parse_at(text, prior_query, revision, &context, grounding.and_then(|g|g.reference_resolution.as_ref())), Some(ParsedInput::Query(q)) if q == self.query)
            || self.memory.premises.last().is_some_and(|p| {
                p.source_text == text
                    && p.answer_binding
                        .as_ref()
                        .is_some_and(|b| b.query == self.query)
            })
    }

    pub fn answer_disposition(&self) -> crate::discourse_qa::DiscourseAnswerDispositionIR {
        use crate::discourse_qa::DiscourseAnswerDispositionIR as D;
        match self.decision.verdict {
            WorldVerdictIR::Supported | WorldVerdictIR::Refuted => D::AnsweredFromDialogueRecords,
            WorldVerdictIR::Conflict => D::ConflictingDialogueRecords,
            WorldVerdictIR::Unknown | WorldVerdictIR::ResourceBound => {
                D::DialogueTruthNotEstablished
            }
        }
    }

    pub fn into_answer(
        self,
        text: &str,
        language: crate::language_knowledge::LanguageCodeIR,
    ) -> Result<crate::discourse_qa::DiscourseAnswerIR, String> {
        let mut answer = crate::discourse_qa::DiscourseQaEngine.unanswered(text, language);
        answer.claims.clear();
        answer.disposition = self.answer_disposition();
        answer.realized_text =
            crate::generative_language::generate_world_decision(language, &self)?
                .morphology
                .realized_text;
        answer.world_reasoning = Some(self);
        Ok(answer)
    }

    pub fn validate(&self) -> bool {
        let turn = self.memory.latest_turn();
        self.memory.validate(turn)
            && deliberate_world(&self.memory, &self.query).as_ref() == Ok(self)
    }
}

/// The adapter creates a goal-directed working set. All search, state changes,
/// competing hypotheses and counterfactual simulation are performed by the core.
pub fn deliberate_world(
    memory: &DialogueWorldIR,
    query: &WorldQueryIR,
) -> Result<WorldReasoningIR, String> {
    let turn = memory.latest_turn();
    if !memory.validate(turn) {
        return Err("INVALID_WORLD_MEMORY".into());
    }
    deliberate_world_inner(memory, query)
}

fn deliberate_world_inner(
    memory: &DialogueWorldIR,
    query: &WorldQueryIR,
) -> Result<WorldReasoningIR, String> {
    if !valid_atom(&query.target.0, &memory.vocabulary) {
        return Err("INVALID_WORLD_QUERY".into());
    }
    let mut atoms = BTreeMap::from([(query.target.0.id(), query.target.0.clone())]);
    let mut rules_by_effect = BTreeMap::<String, Vec<usize>>::new();
    for (index, rule) in memory.implications.iter().enumerate() {
        rules_by_effect
            .entry(rule.effect.0.id())
            .or_default()
            .push(index);
    }
    let mut needed = BTreeSet::from([query.target.0.id()]);
    let mut frontier = vec![query.target.0.id()];
    let mut selected = BTreeSet::new();
    while let Some(id) = frontier.pop() {
        for &index in rules_by_effect.get(&id).into_iter().flatten() {
            if !selected.insert(index) {
                continue;
            }
            let rule = &memory.implications[index];
            atoms.insert(rule.effect.0.id(), rule.effect.0.clone());
            for (atom, _) in &rule.prerequisites {
                atoms.insert(atom.id(), atom.clone());
                if needed.insert(atom.id()) {
                    frontier.push(atom.id());
                }
            }
        }
    }
    if needed.len() > 64 {
        return Err("WORLD_WORKING_SET_BOUND".into());
    }
    let mut evidence = memory
        .premises
        .iter()
        .enumerate()
        .filter(|(_, p)| p.active && needed.contains(&p.atom.id()))
        .map(|(i, p)| {
            atoms.insert(p.atom.id(), p.atom.clone());
            EvidenceIR {
                evidence_id: format!("E{i}"),
                literal: p.atom.literal(p.value),
                // Reliability of the supplied premise inside this model, NOT
                // a calibrated claim that a user's statement is true in reality.
                reliability_millis: 1000,
                source_ref: format!("USER_PREMISE:{}:{}", p.introduced_turn, p.source_sha256),
            }
        })
        .collect::<Vec<_>>();
    if let Some((atom, value)) = &query.assumption {
        if !valid_atom(atom, &memory.vocabulary) {
            return Err("INVALID_WORLD_ASSUMPTION".into());
        }
        atoms.insert(atom.id(), atom.clone());
        evidence.retain(|e| e.literal.proposition_id != atom.id());
        evidence.push(EvidenceIR {
            evidence_id: "HYPOTHETICAL".into(),
            literal: atom.literal(*value),
            reliability_millis: 1000,
            source_ref: "QUERY_LOCAL_INTERVENTION_NOT_OBSERVATION".into(),
        });
    }
    let mut mechanisms = selected
        .into_iter()
        .map(|index| {
            let r = &memory.implications[index];
            CausalMechanismIR {
                mechanism_id: format!("R{index}"),
                kind: MechanismKindIR::Inference,
                prerequisites: r.prerequisites.iter().map(|(a, v)| a.literal(*v)).collect(),
                effects: vec![r.effect.0.literal(r.effect.1)],
                observes: vec![],
                authority: ActionAuthorityIR::InternalInference,
                authorized: true,
                reversible: true,
                recovery_reference: None,
                cost_millis: 1,
                risk_millis: 0,
                provenance_refs: vec![format!(
                    "USER_CONDITIONAL_PREMISE:{}:{}",
                    r.introduced_turn, r.source_sha256
                )],
            }
        })
        .collect::<Vec<_>>();
    let known = evidence
        .iter()
        .map(|e| e.literal.proposition_id.as_str())
        .collect::<BTreeSet<_>>();
    // Asking about a missing leaf premise can discriminate the core's competing
    // explanations. It does not assert a value or execute a sensor/tool.
    for id in needed
        .iter()
        .filter(|id| !known.contains(id.as_str()) && !rules_by_effect.contains_key(*id))
        .take(16)
    {
        mechanisms.push(CausalMechanismIR {
            mechanism_id: format!("Q_{id}"),
            kind: MechanismKindIR::Diagnostic,
            prerequisites: vec![],
            effects: vec![],
            observes: vec![id.clone()],
            authority: ActionAuthorityIR::ReadOnlyObservation,
            authorized: true,
            reversible: true,
            recovery_reference: None,
            cost_millis: 1,
            risk_millis: 0,
            provenance_refs: vec!["UNRESOLVED_PREMISE_QUERY".into()],
        });
    }
    let mut requests = Vec::new();
    let mut deliberations = Vec::new();
    let mut proof_mechanisms = mechanisms
        .iter()
        .filter(|m| m.kind != MechanismKindIR::Diagnostic)
        .cloned()
        .map(encoded_mechanism)
        .collect::<Vec<_>>();
    let possible_supports = evidence
        .iter()
        .map(|e| encode_support(&e.literal).proposition_id)
        .chain(
            proof_mechanisms
                .iter()
                .flat_map(|m| m.effects.iter().map(|e| e.proposition_id.clone())),
        )
        .collect::<BTreeSet<_>>();
    // Ask the same core whether *any relevant proposition* has both proofs.
    // A conjunction with two signed supports is logic compilation, not a
    // second search engine and not a language-dependent answer rule.
    for (index, id) in needed.iter().enumerate() {
        if ![true, false].into_iter().all(|value| {
            possible_supports.contains(
                &encode_support(&LiteralIR {
                    proposition_id: id.clone(),
                    value,
                })
                .proposition_id,
            )
        }) {
            continue;
        }
        proof_mechanisms.push(CausalMechanismIR {
            mechanism_id: format!("K{index}"),
            kind: MechanismKindIR::Inference,
            prerequisites: vec![
                encode_support(&LiteralIR {
                    proposition_id: id.clone(),
                    value: true,
                }),
                encode_support(&LiteralIR {
                    proposition_id: id.clone(),
                    value: false,
                }),
            ],
            effects: vec![LiteralIR {
                proposition_id: "WORLD_CONFLICT".into(),
                value: true,
            }],
            observes: vec![],
            authority: ActionAuthorityIR::InternalInference,
            authorized: true,
            reversible: true,
            recovery_reference: None,
            cost_millis: 1,
            risk_millis: 0,
            provenance_refs: vec!["LOGIC:JOINT_SUPPORT_AND_REFUTATION".into()],
        });
    }
    let goals = [
        encode_support(&query.target.0.literal(query.target.1)),
        encode_support(&query.target.0.literal(!query.target.1)),
        LiteralIR {
            proposition_id: "WORLD_CONFLICT".into(),
            value: true,
        },
    ];
    for (index, goal) in goals.into_iter().enumerate() {
        let request = DeliberationRequestIR {
            schema: DELIBERATION_REQUEST_SCHEMA.into(),
            request_id: format!("WORLD_{index}"),
            subject: query.target.0.id(),
            evidence: evidence
                .iter()
                .cloned()
                .map(|mut e| {
                    e.literal = encode_support(&e.literal);
                    e
                })
                .collect(),
            mechanisms: proof_mechanisms.clone(),
            goals: vec![goal],
            authority_envelope: AuthorityEnvelopeIR {
                allow_internal_inference: true,
                allow_read_only_observation: true,
                allow_reversible_mutation: false,
                allow_irreversible_mutation: false,
                mutation_scope_id: None,
                ..Default::default()
            },
            immutable_constraints: vec![],
            max_depth: 16,
            beam_width: 32,
            max_hypotheses: 32,
            max_counterfactuals: 64,
        };
        let request = goal_working_set(request);
        deliberations.push(
            DeliberationEngine
                .deliberate(&request)
                .map_err(|e| format!("{e:?}"))?,
        );
        requests.push(request);
    }
    let reaches = |d: &DeliberationIR| {
        matches!(
            d.disposition,
            DeliberationDispositionIR::GoalAlreadySatisfied
                | DeliberationDispositionIR::GoalReachable
        )
    };
    let support = reaches(&deliberations[0]);
    let oppose = reaches(&deliberations[1]);
    let conflict = reaches(&deliberations[2]);
    // A truncated opposing search cannot justify certainty in the other direction.
    let bounded = deliberations
        .iter()
        .any(|d| d.disposition == DeliberationDispositionIR::ResourceBoundReached);
    let verdict = if conflict || (support && oppose) {
        WorldVerdictIR::Conflict
    } else if bounded {
        WorldVerdictIR::ResourceBound
    } else if support {
        WorldVerdictIR::Supported
    } else if oppose {
        WorldVerdictIR::Refuted
    } else {
        WorldVerdictIR::Unknown
    };
    let chosen = if oppose && !support { 1 } else { 0 };
    let conclusion =
        matches!(verdict, WorldVerdictIR::Supported | WorldVerdictIR::Refuted).then(|| {
            query.target.0.literal(if chosen == 0 {
                query.target.1
            } else {
                !query.target.1
            })
        });
    // Separate proof search from information acquisition. In the core's public
    // disposition, DiagnosticRequired precedes ResourceBoundReached; including
    // diagnostics in proof search could hide an incomplete opposing search.
    if verdict == WorldVerdictIR::Unknown {
        let mut diagnostic_request = requests[0].clone();
        diagnostic_request.request_id = "WORLD_DIAGNOSTIC".into();
        diagnostic_request.mechanisms = mechanisms
            .iter()
            .filter(|m| m.kind == MechanismKindIR::Diagnostic)
            .cloned()
            .collect();
        deliberations.push(
            DeliberationEngine
                .deliberate(&diagnostic_request)
                .map_err(|e| format!("{e:?}"))?,
        );
        requests.push(diagnostic_request);
    }
    // Information acquisition must make progress: never return the user's
    // original goal as a question when no discriminating premise is available.
    let question = if verdict == WorldVerdictIR::Unknown {
        deliberations
            .last()
            .and_then(|d| d.recommended_diagnostic_id.as_ref())
            .and_then(|id| mechanisms.iter().find(|m| &m.mechanism_id == id))
            .and_then(|m| m.observes.first())
            .filter(|id| **id != query.target.0.id())
            .cloned()
            .map(|diagnostic| LiteralIR {
                proposition_id: diagnostic,
                value: true,
            })
    } else {
        None
    };
    let proof_mechanism_ids = if conclusion.is_some() {
        deliberations[chosen]
            .selected_plan
            .as_ref()
            .map(|p| p.mechanism_ids.clone())
            .unwrap_or_default()
    } else {
        vec![]
    };
    let decision = WorldDecisionIR {
        verdict,
        target: query.target.0.literal(query.target.1),
        conclusion,
        question,
        proof_mechanism_ids,
        premise_evidence_ids: evidence.iter().map(|e| e.evidence_id.clone()).collect(),
        hypothetical: query.assumption.is_some(),
        external_action_authorized: false,
    };
    let semantic_decision_sha256 = hash(&decision);
    let utterance_plan = plan_utterance(query, &decision, &requests)?;
    Ok(WorldReasoningIR {
        memory: memory.clone(),
        query: query.clone(),
        requests,
        deliberations,
        atoms,
        decision,
        semantic_decision_sha256,
        utterance_plan,
    })
}

fn hash<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("world IR serializes"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_vocabulary::{
        WorldLexemeIR, WorldLexicalGrammarIR as G, WorldPredicateArityIR as A,
        WorldPredicateSpecIR, WorldVocabularyUpdateIR,
    };
    use crate::{
        CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    };

    fn vocabulary_update() -> WorldVocabularyUpdateIR {
        let specs = [
            ("W_USER_1", A::Unary),
            ("W_USER_2", A::Binary),
            ("W_USER_3", A::Binary),
        ];
        let entries = [
            (
                "opaque.en",
                "W_USER_1",
                LanguageCodeIR::English,
                "muru",
                G::Copular,
            ),
            (
                "opaque.ko",
                "W_USER_1",
                LanguageCodeIR::Korean,
                "무루",
                G::Copular,
            ),
            (
                "relation.en",
                "W_USER_2",
                LanguageCodeIR::English,
                "depend on",
                G::EnglishRegularVerb,
            ),
            (
                "relation.ko",
                "W_USER_2",
                LanguageCodeIR::Korean,
                "의존",
                G::KoreanHadaLocative,
            ),
            (
                "transitive.en",
                "W_USER_3",
                LanguageCodeIR::English,
                "trust",
                G::EnglishRegularVerb,
            ),
            (
                "transitive.ko",
                "W_USER_3",
                LanguageCodeIR::Korean,
                "신뢰",
                G::KoreanHadaAccusative,
            ),
        ];
        WorldVocabularyUpdateIR {
            predicates: specs
                .into_iter()
                .map(|(id, arity)| WorldPredicateSpecIR {
                    predicate_id: id.into(),
                    arity,
                })
                .collect(),
            aliases: entries
                .into_iter()
                .map(|(alias, id, language, root, grammar)| WorldLexemeIR {
                    alias_id: alias.into(),
                    predicate_id: id.into(),
                    language,
                    root: root.into(),
                    grammar,
                })
                .collect(),
            remove_alias_ids: vec![],
        }
    }

    fn registered_memory(texts: &[&str]) -> DialogueWorldIR {
        let mut m = DialogueWorldIR {
            vocabulary: WorldVocabularyIR::default()
                .updated(&vocabulary_update())
                .unwrap(),
            ..Default::default()
        };
        for (i, text) in texts.iter().enumerate() {
            let p = m.prepare(text, i as u64 + 1).unwrap();
            assert!(p.recognized, "unrecognized: {text}");
            m = p.memory;
        }
        m
    }

    #[test]
    fn world_fresh_registered_roots_use_one_grammar_and_one_core() {
        // Generated lexical identities are deliberately absent from runtime
        // source knowledge. This is a structural test, not a blind benchmark.
        for index in 0..12 {
            let root = format!("nuvu{}", char::from(b'a' + index));
            let korean = format!(
                "무루{}",
                char::from_u32('가' as u32 + u32::from(index)).unwrap()
            );
            let mut update = vocabulary_update();
            update.aliases[0].root = root.clone();
            update.aliases[1].root = korean.clone();
            let mut m = DialogueWorldIR {
                vocabulary: WorldVocabularyIR::default().updated(&update).unwrap(),
                ..Default::default()
            };
            for (i, text) in [
                format!("alpha is {root}."),
                format!("If alpha is {root}, then beta trusts gamma."),
            ]
            .iter()
            .enumerate()
            {
                let p = m.prepare(text, i as u64 + 1).unwrap();
                assert!(p.recognized);
                m = p.memory;
            }
            let r = reason(&m, "Why does beta trust gamma?");
            assert_eq!(r.decision.verdict, WorldVerdictIR::Supported);
            assert_eq!(r.decision.proof_mechanism_ids.len(), 1);
            let g = crate::generative_language::generate_world_decision(LanguageCodeIR::Korean, &r)
                .unwrap();
            assert!(g.validate());
            assert!(g.morphology.realized_text.contains(&korean));
            let ending = if index == 0 { "야." } else { "이야." };
            assert!(g
                .morphology
                .realized_text
                .contains(&format!("{korean}{ending}")));
            assert_eq!(
                reason(&m, &format!("alpha는 {korean}인가?"))
                    .decision
                    .verdict,
                WorldVerdictIR::Supported
            );
            let particle = if index == 0 { "가" } else { "이" };
            m = m
                .prepare(&format!("정정: alpha는 {korean}{particle} 아니야."), 3)
                .unwrap()
                .memory;
            let negative = reason(&m, &format!("alpha는 {korean}인가?"));
            assert_eq!(negative.decision.verdict, WorldVerdictIR::Refuted);
            let realized = crate::generative_language::generate_world_decision(
                LanguageCodeIR::Korean,
                &negative,
            )
            .unwrap();
            assert!(realized
                .morphology
                .realized_text
                .contains(&format!("{korean}{particle} 아니야.")));
        }
    }

    #[test]
    fn world_vocabulary_bounds_and_output_gap_are_atomic() {
        let mut v = WorldVocabularyIR::default()
            .updated(&vocabulary_update())
            .unwrap();
        let mut prior_id = "opaque.en".to_string();
        for i in 0..30 {
            let mut alias = vocabulary_update().aliases[0].clone();
            alias.alias_id = format!("rename{i}");
            v = v
                .updated(&WorldVocabularyUpdateIR {
                    remove_alias_ids: vec![prior_id],
                    aliases: vec![alias.clone()],
                    ..Default::default()
                })
                .unwrap();
            prior_id = alias.alias_id;
        }
        assert_eq!(v.lexical_history.len(), 32);
        let before = v.clone();
        assert!(v
            .updated(&WorldVocabularyUpdateIR {
                remove_alias_ids: vec![prior_id],
                ..Default::default()
            })
            .is_err());
        assert_eq!(before, v);
        assert_eq!(v.updated(&WorldVocabularyUpdateIR::default()).unwrap(), v);
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut update = vocabulary_update();
        update.aliases.retain(|a| a.alias_id != "opaque.ko");
        assert!(
            api.execute_command(
                crate::cognitive::CognitiveApiCommandIR::UpdateWorldVocabulary {
                    conversation_id: "OUTPUT-GAP".into(),
                    update
                }
            )
            .ok
        );
        let before = api.conversation_state("OUTPUT-GAP").unwrap().clone();
        assert!(api
            .process_conversation_turn(&request(
                "OUTPUT-GAP",
                1,
                "alpha is muru.",
                LanguageCodeIR::Korean
            ))
            .is_err());
        assert_eq!(&before, api.conversation_state("OUTPUT-GAP").unwrap());
        assert!(api
            .process_conversation_turn(&request(
                "OUTPUT-GAP",
                1,
                "alpha is muru.",
                LanguageCodeIR::English
            ))
            .is_ok());
    }

    #[test]
    fn world_registered_relations_compose_without_changing_reasoner() {
        for entity in ["cedar72", "node91", "장치83"] {
            let m = registered_memory(&[
                &format!("{entity} is muru."),
                &format!("If {entity} is muru, then {entity} depends on relay."),
                &format!(
                    "If {entity} depends on relay and relay is active, then terminal is safe."
                ),
                "relay is active.",
            ]);
            let result = reason(&m, "Why is terminal safe?");
            assert_eq!(result.decision.verdict, WorldVerdictIR::Supported);
            assert_eq!(result.decision.proof_mechanism_ids.len(), 2);
            for lang in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
                let generated =
                    crate::generative_language::generate_world_decision(lang, &result).unwrap();
                assert!(
                    generated.validate(),
                    "{}",
                    generated.morphology.realized_text
                );
                assert_eq!(generated.verification.unsupported_claims, 0);
                assert!(generated.morphology.realized_text.contains(entity));
                assert!(generated.morphology.realized_text.contains(
                    if lang == LanguageCodeIR::Korean {
                        "의존"
                    } else {
                        "depends on"
                    }
                ));
            }
            assert_eq!(
                reason(&m, &format!("Does relay depend on {entity}?"))
                    .decision
                    .verdict,
                WorldVerdictIR::Unknown
            );
        }
    }

    #[test]
    fn world_registered_ko_en_same_ids_and_negative_relation_matrix() {
        let en = registered_memory(&[
            "alpha is muru.",
            "If alpha is muru, then alpha depends on beta.",
            "If alpha depends on beta, then beta trusts gamma.",
        ]);
        let ko = registered_memory(&[
            "alpha는 무루다.",
            "alpha는 무루이면 alpha는 beta에 의존한다.",
            "alpha는 beta에 의존하면 beta는 gamma를 신뢰한다.",
        ]);
        let er = reason(&en, "Does beta trust gamma?");
        let kr = reason(&ko, "beta는 gamma를 신뢰하나요?");
        assert_eq!(er.semantic_decision_sha256, kr.semantic_decision_sha256);
        assert_eq!(er.decision.verdict, WorldVerdictIR::Supported);
        for (positive, negative, query) in [
            (
                "alpha depends on beta.",
                "alpha does not depend on beta.",
                "Does alpha depend on beta?",
            ),
            (
                "alpha는 beta에 의존한다.",
                "alpha는 beta에 의존하지 않는다.",
                "alpha는 beta에 의존하나요?",
            ),
            (
                "alpha trusts beta.",
                "alpha does not trust beta.",
                "Does alpha trust beta?",
            ),
            (
                "alpha는 beta를 신뢰한다.",
                "alpha는 beta를 신뢰하지 않는다.",
                "alpha는 beta를 신뢰하나요?",
            ),
        ] {
            for (premises, verdict) in [
                (vec![], WorldVerdictIR::Unknown),
                (vec![positive], WorldVerdictIR::Supported),
                (vec![negative], WorldVerdictIR::Refuted),
                (vec![positive, negative], WorldVerdictIR::Conflict),
            ] {
                let result = reason(&registered_memory(&premises), query);
                assert_eq!(result.decision.verdict, verdict);
                for lang in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
                    let g =
                        crate::generative_language::generate_world_decision(lang, &result).unwrap();
                    assert!(g.validate(), "{}", g.morphology.realized_text);
                    if verdict == WorldVerdictIR::Refuted {
                        assert!(g.morphology.realized_text.contains(
                            if lang == LanguageCodeIR::Korean {
                                "하지 않아"
                            } else {
                                "does not"
                            }
                        ));
                    }
                }
            }
        }
    }

    #[test]
    fn world_alias_revision_preserves_memory_and_unnamed_reasoning() {
        let mut m = registered_memory(&["alpha is muru.", "If alpha is muru, then beta is safe."]);
        let before = reason(&m, "Is beta safe?");
        let hash = m.vocabulary.semantic_sha256();
        m.vocabulary = m
            .vocabulary
            .updated(&WorldVocabularyUpdateIR {
                remove_alias_ids: vec!["opaque.en".into(), "opaque.ko".into()],
                ..Default::default()
            })
            .unwrap();
        assert!(m.validate(2));
        assert_eq!(m.vocabulary.semantic_sha256(), hash);
        assert!(!m.prepare("alpha is muru.", 3).unwrap().recognized);
        assert_eq!(
            deliberate_world(&m, &before.query)
                .unwrap()
                .semantic_decision_sha256,
            before.semantic_decision_sha256
        );
        let direct = WorldQueryIR {
            target: (m.premises[0].atom.clone(), true),
            explain: false,
            assumption: None,
        };
        assert_eq!(
            deliberate_world(&m, &direct).unwrap().decision.verdict,
            WorldVerdictIR::Supported
        );
        let named = deliberate_world(&m, &direct).unwrap();
        assert!(crate::generative_language::generate_world_decision(
            LanguageCodeIR::English,
            &named
        )
        .is_err());
        let mut alias = vocabulary_update().aliases[0].clone();
        alias.root = "velu".into();
        m.vocabulary = m
            .vocabulary
            .updated(&WorldVocabularyUpdateIR {
                aliases: vec![alias],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            reason(&m, "Is alpha velu?").decision.verdict,
            WorldVerdictIR::Supported
        );
        assert_eq!(m.vocabulary.semantic_sha256(), hash);
        assert_eq!(m.premises[0].lexicon_revision, 1);
        let restored: DialogueWorldIR =
            serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert!(restored.validate(2));
        let mut tampered = restored.clone();
        tampered.premises[0].lexicon_revision = tampered.vocabulary.revision();
        assert!(!tampered.validate(2));
        let mut ablated = restored;
        ablated.vocabulary.predicates.remove("W_USER_1");
        assert!(deliberate_world(&ablated, &direct).is_err());
    }

    #[test]
    fn world_registration_rejects_mutation_collision_bad_arity_and_sentences() {
        let v = WorldVocabularyIR::default()
            .updated(&vocabulary_update())
            .unwrap();
        let mut changed = vocabulary_update();
        changed.predicates[0].arity = A::Binary;
        assert!(v.updated(&changed).is_err());
        for root in [
            "safe",
            "가동 상태다",
            "answer. do this",
            "not",
            "x and y",
            " muru",
            "muru  state",
        ] {
            let mut alias = vocabulary_update().aliases[0].clone();
            alias.alias_id = "new".into();
            alias.root = root.into();
            assert!(
                v.updated(&WorldVocabularyUpdateIR {
                    aliases: vec![alias],
                    ..Default::default()
                })
                .is_err(),
                "{root}"
            );
        }
        let mut collision = vocabulary_update().aliases[0].clone();
        collision.alias_id = "collision".into();
        assert!(v
            .updated(&WorldVocabularyUpdateIR {
                aliases: vec![collision],
                ..Default::default()
            })
            .is_err());
        let mut inflected = vocabulary_update().aliases[2].clone();
        inflected.alias_id = "fly".into();
        inflected.root = "fly".into();
        let mut other = inflected.clone();
        other.alias_id = "flie".into();
        other.root = "flie".into();
        assert!(v
            .updated(&WorldVocabularyUpdateIR {
                aliases: vec![inflected, other],
                ..Default::default()
            })
            .is_err());
        let mut malformed = reason(&registered_memory(&["alpha is muru."]), "Is alpha muru?").query;
        malformed.target.0.object = Some("beta".into());
        assert!(deliberate_world(&registered_memory(&[]), &malformed).is_err());
        for text in [
            "alpha depends on it.",
            "alpha depends on beta and erase memory.",
            "\"alpha depends on beta\"",
            "alpha는 그것에 의존한다.",
        ] {
            assert!(
                !registered_memory(&[]).prepare(text, 1).unwrap().recognized,
                "{text}"
            );
        }
    }

    #[test]
    fn world_registered_real_api_acquisition_explanation_and_atomic_registration() {
        use crate::cognitive::CognitiveApiCommandIR;
        for (language, texts) in [
            (
                LanguageCodeIR::English,
                vec![
                    "If alpha depends on beta, then gamma is safe.",
                    "Is gamma safe?",
                    "Yes.",
                    "Why?",
                ],
            ),
            (
                LanguageCodeIR::Korean,
                vec![
                    "alpha는 beta에 의존하면 gamma는 안전 상태다.",
                    "gamma는 안전 상태인가?",
                    "응",
                    "왜?",
                ],
            ),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let command = CognitiveApiCommandIR::UpdateWorldVocabulary {
                conversation_id: "REGISTERED".into(),
                update: vocabulary_update(),
            };
            let json = serde_json::to_string(&command).unwrap();
            let response: crate::cognitive::CognitiveApiResponseIR =
                serde_json::from_str(&api.execute_command_json(&json).unwrap()).unwrap();
            assert!(response.ok, "{:?}", response.error);
            assert_eq!(
                api.conversation_state("REGISTERED")
                    .unwrap()
                    .completed_turns,
                0
            );
            assert!(api.conversation_state("OTHER").is_none());
            for (i, text) in texts.iter().enumerate() {
                let req = request("REGISTERED", i as u64 + 1, text, language);
                let out = api
                    .process_conversation_turn(&req)
                    .unwrap_or_else(|e| panic!("{text}: {e:?}"));
                assert!(out.validate_against(&req));
                println!("REGISTERED {text} => {}", out.output.text);
                assert!(out
                    .conversation_state
                    .action_state_ledger
                    .records
                    .is_empty());
                if i == 1 {
                    assert!(out.output.text.contains("beta"));
                }
                if i >= 2 {
                    let r = out
                        .discourse_answer
                        .as_ref()
                        .unwrap()
                        .world_reasoning
                        .as_ref()
                        .unwrap();
                    assert_eq!(r.decision.verdict, WorldVerdictIR::Supported);
                    assert_eq!(r.memory.premises[0].atom.object.as_deref(), Some("beta"));
                }
            }
            let before = api.conversation_state("REGISTERED").unwrap().clone();
            let mut bad = vocabulary_update();
            bad.predicates[1].arity = A::Unary;
            let failure = api.execute_command(CognitiveApiCommandIR::UpdateWorldVocabulary {
                conversation_id: "REGISTERED".into(),
                update: bad,
            });
            assert!(!failure.ok);
            assert_eq!(&before, api.conversation_state("REGISTERED").unwrap());
            // Alias-only changes preserve a prior elicited reply and its core question receipt.
            let remove = WorldVocabularyUpdateIR {
                remove_alias_ids: vec!["relation.en".into(), "relation.ko".into()],
                ..Default::default()
            };
            assert!(
                api.execute_command(CognitiveApiCommandIR::UpdateWorldVocabulary {
                    conversation_id: "REGISTERED".into(),
                    update: remove
                })
                .ok
            );
            assert_eq!(
                api.conversation_state("REGISTERED")
                    .unwrap()
                    .completed_turns,
                4
            );
            assert!(api
                .conversation_state("REGISTERED")
                .unwrap()
                .dialogue_world
                .validate(4));
        }
    }

    fn memory(texts: &[&str]) -> DialogueWorldIR {
        let mut memory = DialogueWorldIR::default();
        for (index, text) in texts.iter().enumerate() {
            let prepared = memory.prepare(text, index as u64 + 1).unwrap();
            assert!(prepared.recognized, "not parsed: {text}");
            memory = prepared.memory;
        }
        memory
    }
    fn reason(memory: &DialogueWorldIR, text: &str) -> WorldReasoningIR {
        let turn = memory.latest_turn() + 1;
        let p = memory.prepare(text, turn).unwrap();
        deliberate_world(&p.memory, &p.query.unwrap()).unwrap()
    }
    fn request(
        id: &str,
        turn: u64,
        text: &str,
        language: LanguageCodeIR,
    ) -> ConversationTurnRequestIR {
        ConversationTurnRequestIR {
            schema: crate::conversation::CONVERSATION_TURN_REQUEST_SCHEMA.into(),
            conversation_id: id.into(),
            turn_index: turn,
            request_id: format!("WORLD-{turn}"),
            modality: ConversationInputModalityIR::Text,
            raw_text: text.into(),
            input_confidence_millis: 1000,
            alternatives: vec![],
            output_language: Some(language),
            context_tags: vec![],
            max_plan_steps: 16,
        }
    }

    #[test]
    fn world_conversation_personal_state_and_elliptical_corrections() {
        for (language, texts) in [
            (
                LanguageCodeIR::Korean,
                vec![
                    "나 피곤해.",
                    "한가해?",
                    "아니, 한가하지 않아.",
                    "피곤해?",
                    "왜?",
                ],
            ),
            (
                LanguageCodeIR::English,
                vec!["I am tired.", "Free?", "No, not free.", "Tired?", "Why?"],
            ),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            for (i, text) in texts.iter().enumerate() {
                let input = request("PERSONAL", i as u64 + 1, text, language);
                let out = api
                    .process_conversation_turn(&input)
                    .unwrap_or_else(|e| panic!("{text}: {e:?}"));
                assert!(out.validate_against(&input), "{text}");
                println!("PERSONAL {text} => {}", out.output.text);
                assert!(out
                    .conversation_state
                    .action_state_ledger
                    .records
                    .is_empty());
                if i == 0 {
                    assert_eq!(
                        out.conversation_state.dialogue_world.premises[0]
                            .atom
                            .entity,
                        "__user__"
                    );
                    assert!(out
                        .output
                        .text
                        .contains(if language == LanguageCodeIR::Korean {
                            "피곤하구나"
                        } else {
                            "you are tired"
                        }));
                    assert!(!out.output.text.contains("__user__"));
                }
                if i == 1 || i >= 3 {
                    let w = out
                        .discourse_answer
                        .as_ref()
                        .and_then(|a| a.world_reasoning.as_ref())
                        .unwrap_or_else(|| panic!("world lost: {}", out.output.text));
                    assert_eq!(
                        w.decision.verdict,
                        if i == 1 {
                            WorldVerdictIR::Unknown
                        } else {
                            WorldVerdictIR::Supported
                        }
                    );
                    if i == 4 {
                        assert_eq!(w.utterance_plan.moves.len(), 1);
                        assert_eq!(
                            w.utterance_plan.moves[0].purpose,
                            WorldMovePurposeIR::CauseUnknown
                        );
                        assert!(out
                            .output
                            .text
                            .contains(if language == LanguageCodeIR::Korean {
                                "네가 피곤한 이유는 아직 모르겠어"
                            } else {
                                "I don't know why you are tired"
                            }));
                    }
                }
            }
            let state = &api.conversation_state("PERSONAL").unwrap().dialogue_world;
            let mut tampered = state.clone();
            tampered.premises[1]
                .grounding_context
                .focus
                .as_mut()
                .unwrap()
                .0
                .entity = "somebody_else".into();
            assert!(!tampered.validate(5));
            let restored: DialogueWorldIR =
                serde_json::from_str(&serde_json::to_string(state).unwrap()).unwrap();
            assert!(restored.validate(5));
        }
    }

    #[test]
    fn world_conversation_focus_contrast_and_filled_information_slots() {
        for (language, texts) in [
            (
                LanguageCodeIR::Korean,
                vec![
                    "lamp가 가동 상태이면 gate는 열림 상태다.",
                    "gate는 열림 상태인가?",
                    "가동 상태야.",
                    "왜?",
                    "고마워!",
                    "다른문은?",
                ],
            ),
            (
                LanguageCodeIR::English,
                vec![
                    "If lamp is active, then gate is open.",
                    "Is gate open?",
                    "Active.",
                    "Why?",
                    "Thanks!",
                    "What about sidegate?",
                ],
            ),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            for (i, text) in texts.iter().enumerate() {
                let input = request("SLOT", i as u64 + 1, text, language);
                let out = api
                    .process_conversation_turn(&input)
                    .unwrap_or_else(|e| panic!("{text}: {e:?}"));
                assert!(out.validate_against(&input));
                println!("SLOT {text} => {}", out.output.text);
                if i == 1 {
                    assert!(out.output.text.contains("lamp"));
                }
                if i == 2 || i == 3 {
                    let w = out
                        .discourse_answer
                        .as_ref()
                        .unwrap()
                        .world_reasoning
                        .as_ref()
                        .unwrap();
                    assert_eq!(w.decision.verdict, WorldVerdictIR::Supported);
                    assert_eq!(w.memory.premises[0].atom.entity, "lamp");
                    assert!(w.memory.premises[0].answer_binding.is_some());
                }
                if i == 5 {
                    let w = out
                        .discourse_answer
                        .as_ref()
                        .unwrap()
                        .world_reasoning
                        .as_ref()
                        .unwrap();
                    assert_eq!(w.decision.verdict, WorldVerdictIR::Unknown);
                    assert_ne!(w.query.target.0.entity, "gate");
                    assert_eq!(w.query.target.0.property, WorldPropertyIR::Open);
                }
                assert!(out
                    .conversation_state
                    .action_state_ledger
                    .records
                    .is_empty());
            }
        }
    }

    #[test]
    fn world_conversation_ambiguous_reference_is_asked_then_bound() {
        use crate::cognitive::CognitiveApiCommandIR;
        for (language, texts) in [
            (
                LanguageCodeIR::English,
                vec![
                    "alpha depends on beta.",
                    "Is it safe?",
                    "beta",
                    "It is safe.",
                    "Why?",
                ],
            ),
            (
                LanguageCodeIR::Korean,
                vec![
                    "alpha는 beta에 의존한다.",
                    "그것은 안전 상태인가?",
                    "beta야",
                    "그것은 안전 상태다.",
                    "왜?",
                ],
            ),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            assert!(
                api.execute_command(CognitiveApiCommandIR::UpdateWorldVocabulary {
                    conversation_id: "REFERENCE".into(),
                    update: vocabulary_update()
                })
                .ok
            );
            for (i, text) in texts.iter().enumerate() {
                let input = request("REFERENCE", i as u64 + 1, text, language);
                let out = api
                    .process_conversation_turn(&input)
                    .unwrap_or_else(|e| panic!("{text}: {e:?}"));
                assert!(out.validate_against(&input));
                println!("REFERENCE {text} => {}", out.output.text);
                if i == 1 {
                    assert!(out
                        .discourse_answer
                        .as_ref()
                        .unwrap()
                        .world_clarification
                        .is_some());
                    assert!(out.output.text.contains("alpha") && out.output.text.contains("beta"));
                    assert_eq!(out.conversation_state.dialogue_world.premises.len(), 1);
                }
                if i == 2 || i == 4 {
                    let w = out
                        .discourse_answer
                        .as_ref()
                        .unwrap()
                        .world_reasoning
                        .as_ref()
                        .unwrap();
                    assert_eq!(w.query.target.0.entity, "beta");
                    assert_eq!(
                        w.decision.verdict,
                        if i == 2 {
                            WorldVerdictIR::Unknown
                        } else {
                            WorldVerdictIR::Supported
                        }
                    );
                    if i == 2 {
                        let mut bad = w.clone();
                        bad.memory
                            .last_grounding
                            .as_mut()
                            .unwrap()
                            .reference_resolution
                            .as_mut()
                            .unwrap()
                            .selected = "alpha".into();
                        assert!(!bad.matches_question(text));
                    }
                }
                assert!(out
                    .conversation_state
                    .action_state_ledger
                    .records
                    .is_empty());
            }
        }
    }

    #[test]
    fn world_conversation_speaker_references_in_all_argument_roles() {
        use crate::cognitive::CognitiveApiCommandIR;
        for (language, texts) in [
            (
                LanguageCodeIR::English,
                [
                    "I depend on beta.",
                    "Is it safe?",
                    "me",
                    "alpha depends on me.",
                ],
            ),
            (
                LanguageCodeIR::Korean,
                [
                    "나는 beta에 의존해.",
                    "그것은 안전 상태인가?",
                    "나야",
                    "alpha는 나에 의존해.",
                ],
            ),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            assert!(
                api.execute_command(CognitiveApiCommandIR::UpdateWorldVocabulary {
                    conversation_id: "SPEAKER".into(),
                    update: vocabulary_update(),
                })
                .ok
            );
            for (i, text) in texts.iter().enumerate() {
                let input = request("SPEAKER", i as u64 + 1, text, language);
                let out = api.process_conversation_turn(&input).unwrap();
                assert!(out.validate_against(&input));
                assert!(!out.output.text.contains("__user__"));
                if i == 1 {
                    assert!(out
                        .discourse_answer
                        .as_ref()
                        .unwrap()
                        .world_clarification
                        .is_some());
                    assert!(out
                        .output
                        .text
                        .contains(if language == LanguageCodeIR::Korean {
                            "너를"
                        } else {
                            "you"
                        }));
                } else if i == 2 {
                    let world = out
                        .discourse_answer
                        .as_ref()
                        .unwrap()
                        .world_reasoning
                        .as_ref()
                        .unwrap();
                    assert_eq!(world.query.target.0.entity, "__user__");
                    assert_eq!(world.decision.verdict, WorldVerdictIR::Unknown);
                } else if i == 3 {
                    let premise = out
                        .conversation_state
                        .dialogue_world
                        .premises
                        .last()
                        .unwrap();
                    assert_eq!(premise.atom.object.as_deref(), Some("__user__"));
                }
            }
        }
    }

    #[test]
    fn world_conversation_meaning_plan_deduplicates_and_licenses_subject_ellipsis() {
        let m = memory(&[
            "lamp는 가동 상태다.",
            "lamp가 가동 상태이면 lamp는 준비 상태다.",
            "lamp가 준비 상태이면 lamp는 안전 상태다.",
        ]);
        let mut w = reason(&m, "왜 lamp는 안전 상태인가?");
        assert_eq!(w.utterance_plan.moves.len(), 3);
        assert_eq!(
            w.utterance_plan
                .moves
                .iter()
                .filter(|m| Some(&m.proposition) == w.decision.conclusion.as_ref())
                .count(),
            1
        );
        let g = crate::generative_language::generate_world_decision(LanguageCodeIR::Korean, &w)
            .unwrap();
        assert!(g.validate());
        println!("PLAN => {}", g.morphology.realized_text);
        assert_eq!(g.morphology.realized_text.matches("lamp").count(), 1);
        assert_eq!(
            g.morphology
                .tokens
                .iter()
                .filter(|t| t.grammar_rule_id.as_deref() == Some("KO.ZERO_SUBJECT.SHARED_REFERENT"))
                .count(),
            2
        );
        w.utterance_plan.moves.remove(0);
        assert!(!w.validate());
    }

    #[test]
    fn world_conversation_composition_matrix_and_context_ablation() {
        for property in WorldPropertyIR::ALL {
            for (language, full, ellipsis, contrast) in [
                (
                    LanguageCodeIR::English,
                    format!("node is {}.", property.expression(false)),
                    format!("{}?", property.expression(false)),
                    "What about other?",
                ),
                (
                    LanguageCodeIR::Korean,
                    format!("node는 {}다.", property.expression(true)),
                    format!("{}인가?", property.expression(true)),
                    "other는?",
                ),
            ] {
                let m = memory(&[&full]);
                let w = reason(&m, &ellipsis);
                assert_eq!(w.decision.verdict, WorldVerdictIR::Supported);
                assert!(
                    crate::generative_language::generate_world_decision(language, &w)
                        .unwrap()
                        .validate()
                );
                assert_eq!(
                    reason(&w.memory, contrast).decision.verdict,
                    WorldVerdictIR::Unknown
                );
                let mut ablated = m.clone();
                ablated.clear_discourse();
                assert!(!ablated.prepare(&ellipsis, 2).unwrap().recognized);
                let switched = m
                    .prepare("Let's talk about another topic.", 2)
                    .unwrap()
                    .memory;
                assert!(!switched.prepare(&ellipsis, 3).unwrap().recognized);
            }
        }
    }

    #[test]
    fn world_two_step_conjunction_uses_core_search_not_sentence_answers() {
        for entity in ["nival17", "zora83", "물체47"] {
            let seed = format!("{entity} is active.");
            let rule = format!("If {entity} is active and beta is ready, then gamma is available.");
            let world = reason(
                &memory(&[
                    &seed,
                    "beta is ready.",
                    &rule,
                    "If gamma is available, then delta is safe.",
                ]),
                "Is delta safe?",
            );
            assert_eq!(world.decision.verdict, WorldVerdictIR::Supported);
            assert_eq!(world.decision.proof_mechanism_ids.len(), 2);
            assert!(world.deliberations[0].search_states_expanded > 0);
            assert!(world.validate());
            assert!(world
                .requests
                .iter()
                .all(|r| !r.authority_envelope.allow_reversible_mutation
                    && !r.authority_envelope.allow_irreversible_mutation));
            assert!(world
                .deliberations
                .iter()
                .all(|d| d.external_action_execution_events == 0 && d.external_model_calls == 0));
        }
    }

    #[test]
    fn world_missing_evidence_is_not_false_and_produces_a_relevant_question() {
        let m = memory(&[
            "If alpha is active and beta is ready, then gamma is safe.",
            "alpha is active.",
        ]);
        let world = reason(&m, "Is gamma safe?");
        assert_eq!(world.decision.verdict, WorldVerdictIR::Unknown);
        assert!(world.decision.conclusion.is_none());
        assert_eq!(
            world.atoms[&world.decision.question.unwrap().proposition_id].entity,
            "beta"
        );
        let supported = reason(
            &memory(&[
                "If alpha is active, then gamma is safe.",
                "alpha is active.",
            ]),
            "Is gamma safe?",
        );
        let ablated = reason(
            &memory(&["If alpha is active, then gamma is safe."]),
            "Is gamma safe?",
        );
        assert_eq!(supported.decision.verdict, WorldVerdictIR::Supported);
        assert_eq!(ablated.decision.verdict, WorldVerdictIR::Unknown);
    }

    #[test]
    fn world_opposite_proofs_conflict_instead_of_first_match_winning() {
        let m = memory(&[
            "alpha is active.",
            "If alpha is active, then beta is safe.",
            "If alpha is active, then beta is not safe.",
        ]);
        let result = reason(&m, "Is beta safe?");
        assert_eq!(result.decision.verdict, WorldVerdictIR::Conflict);
        for texts in [
            vec![
                "alpha is active.",
                "beta is safe.",
                "If alpha is active, then beta is not safe.",
            ],
            vec![
                "alpha is active.",
                "If alpha is active, then beta is ready.",
                "If alpha is active, then beta is not ready.",
                "If beta is ready, then gamma is safe.",
            ],
        ] {
            let query = if texts.len() == 3 {
                "Is beta safe?"
            } else {
                "Is gamma safe?"
            };
            assert_eq!(
                reason(&memory(&texts), query).decision.verdict,
                WorldVerdictIR::Conflict
            );
        }
        assert!(result.decision.conclusion.is_none());
        let result = reason(
            &memory(&["alpha is active.", "alpha is not active."]),
            "Is alpha active?",
        );
        assert_eq!(result.decision.verdict, WorldVerdictIR::Conflict);
    }

    #[test]
    fn world_correction_revises_memory_but_hypothesis_does_not() {
        let m = memory(&[
            "alpha is active.",
            "If alpha is active, then beta is safe.",
            "Actually, alpha is not active.",
        ]);
        assert!(!m.premises[0].active);
        assert_eq!(
            reason(&m, "Is alpha active?").decision.verdict,
            WorldVerdictIR::Refuted
        );
        assert_eq!(
            reason(&m, "Is beta safe?").decision.verdict,
            WorldVerdictIR::Unknown
        );
        let hypothetical = reason(&m, "Suppose alpha is active. Is beta safe?");
        assert_eq!(hypothetical.decision.verdict, WorldVerdictIR::Supported);
        assert!(hypothetical.decision.hypothetical);
        assert_eq!(hypothetical.memory.premises, m.premises);
        assert_eq!(
            reason(&m, "Is beta safe?").decision.verdict,
            WorldVerdictIR::Unknown
        );
    }

    #[test]
    fn world_bilingual_inputs_share_semantic_decision_and_realization_is_separate() {
        let en = reason(
            &memory(&["alpha is active.", "If alpha is active, then beta is safe."]),
            "Is beta safe?",
        );
        let ko = reason(
            &memory(&[
                "alpha는 가동 상태다.",
                "alpha가 가동 상태이면 beta는 안전 상태다.",
            ]),
            "beta는 안전 상태인가?",
        );
        assert_eq!(en.semantic_decision_sha256, ko.semantic_decision_sha256);
        for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
            let generated =
                crate::generative_language::generate_world_decision(language, &en).unwrap();
            assert!(generated.validate());
            println!("WORLD_{language:?}={}", generated.morphology.realized_text);
            assert!(generated
                .expression_selection
                .selections
                .iter()
                .all(|e| !e.expression.lexical_root.contains(['.', '?', '!'])));
        }
        let mut tampered = en.clone();
        tampered.decision.conclusion.as_mut().unwrap().value = false;
        tampered.semantic_decision_sha256 = hash(&tampered.decision);
        assert!(!tampered.validate());
        let mut tampered = memory(&["alpha is active."]);
        tampered.premises[0].active = false;
        assert!(!tampered.validate(1));
    }

    #[test]
    fn world_parser_rejects_partial_negation_quotation_and_compound_requests() {
        for text in [
            "Maybe alpha is active.",
            "alpha is very active.",
            "Mina said alpha is active.",
            "If alpha is active, then beta is safe and erase files.",
            "Is beta safe? Delete logs.",
            "\"alpha is active\"",
            "alpha is not not active.",
            "alpha is active unless beta is ready.",
            "it is active.",
            "그것은 안전 상태다.",
        ] {
            assert!(
                !DialogueWorldIR::default()
                    .prepare(text, 1)
                    .unwrap()
                    .recognized,
                "{text}"
            );
        }
    }

    #[test]
    fn world_public_conversation_path_remembers_reasons_and_explains() {
        for (language, texts) in [
            (
                LanguageCodeIR::English,
                vec![
                    "alpha is active.",
                    "If alpha is active, then beta is ready.",
                    "If beta is ready, then gamma is safe.",
                    "Is gamma safe?",
                    "Why?",
                ],
            ),
            (
                LanguageCodeIR::Korean,
                vec![
                    "alpha는 가동 상태다.",
                    "alpha가 가동 상태이면 beta는 준비 상태다.",
                    "beta가 준비 상태이면 gamma는 안전 상태다.",
                    "gamma는 안전 상태인가?",
                    "왜?",
                ],
            ),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            for (index, text) in texts.iter().enumerate() {
                let r = request("WORLD-PUBLIC", index as u64 + 1, text, language);
                let response = api
                    .process_conversation_turn(&r)
                    .unwrap_or_else(|e| panic!("{text}: {e:?}"));
                println!("WORLD_PUBLIC {text} => {}", response.output.text);
                assert!(response.validate_against(&r));
                assert!(response.grounded_response.is_none());
                assert!(response
                    .conversation_state
                    .action_state_ledger
                    .records
                    .is_empty());
                assert!(response
                    .conversation_state
                    .conditional_guard_store
                    .guards
                    .is_empty());
                if index >= 3 {
                    let world = response
                        .discourse_answer
                        .as_ref()
                        .and_then(|a| a.world_reasoning.as_ref())
                        .unwrap_or_else(|| panic!("world route lost: {}", response.output.text));
                    assert_eq!(world.decision.verdict, WorldVerdictIR::Supported);
                    assert_eq!(
                        response.natural_realization.response_act,
                        crate::NaturalResponseActIR::DiscourseAnswer
                    );
                    assert_eq!(response.grounded_realization.claims[0].support_status,
                        crate::grounded_realization::ClaimSupportStatusIR::DerivedFromDialogueRecords);
                    assert!(response.output.text.contains("gamma"));
                    if index == 4 {
                        assert!(
                            response.output.text.contains("alpha")
                                && response.output.text.contains("beta")
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn world_path_matrix_varies_semantic_properties_polarity_and_evidence() {
        let mut count = 0;
        for property in WorldPropertyIR::ALL {
            let rule = format!(
                "If nival is {}, then zora is safe.",
                property.expression(false)
            );
            for (premise, expected) in [
                (None, WorldVerdictIR::Unknown),
                (Some(true), WorldVerdictIR::Supported),
                (Some(false), WorldVerdictIR::Unknown),
            ] {
                let premise = premise.map(|value| {
                    format!(
                        "nival is {}{}.",
                        if value { "" } else { "not " },
                        property.expression(false)
                    )
                });
                let mut texts = vec![rule.as_str()];
                if let Some(p) = &premise {
                    texts.push(p);
                }
                let m = memory(&texts);
                for negative_question in [false, true] {
                    let query = format!(
                        "Is zora {}safe?",
                        if negative_question { "not " } else { "" }
                    );
                    let result = reason(&m, &query);
                    let expected = if negative_question && expected == WorldVerdictIR::Supported {
                        WorldVerdictIR::Refuted
                    } else {
                        expected
                    };
                    assert_eq!(result.decision.verdict, expected, "{texts:?} / {query}");
                    for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
                        let answer = result.clone().into_answer(&query, language).unwrap();
                        assert!(answer.validate());
                    }
                    count += 1;
                }
            }
        }
        assert_eq!(count, 48);
    }

    #[test]
    fn world_resource_bound_defers_instead_of_certifying_a_partial_search() {
        let mut strings = vec!["n0 is active.".to_string(), "result is safe.".into()];
        for index in 0..16 {
            strings.push(format!(
                "If n{index} is active, then n{} is active.",
                index + 1
            ));
        }
        strings.push("If n16 is active, then result is not safe.".into());
        let texts = strings.iter().map(String::as_str).collect::<Vec<_>>();
        let result = reason(&memory(&texts), "Is result safe?");
        assert_eq!(result.decision.verdict, WorldVerdictIR::ResourceBound);
        assert!(result.decision.conclusion.is_none());
        assert!(result.decision.question.is_none());
    }

    #[test]
    fn world_ir_works_without_expressions_but_expressions_cannot_replace_the_core() {
        use crate::generative_language::{
            ExpressionNodeStore, GenerativeLanguageCortex, GenerativeLanguageRequestIR,
        };
        let m = memory(&["alpha is active.", "If alpha is active, then beta is safe."]);
        let world = reason(&m, "Is beta safe?");
        let generated =
            crate::generative_language::generate_world_decision(LanguageCodeIR::English, &world)
                .unwrap();
        let empty = ExpressionNodeStore::default();
        let ablated = GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
            meaning: generated.meaning.clone(),
            context: generated.context.clone(),
            expressions: &empty,
        });
        assert!(ablated.is_err());
        assert_eq!(
            deliberate_world(&world.memory, &world.query)
                .unwrap()
                .decision,
            world.decision
        );
        let mut no_rules = m.clone();
        no_rules.implications.clear();
        no_rules.clear_discourse();
        assert_eq!(
            reason(&no_rules, "Is beta safe?").decision.verdict,
            WorldVerdictIR::Unknown
        );
        let mut no_evidence = m;
        no_evidence.premises.clear();
        no_evidence.clear_discourse();
        assert_eq!(
            reason(&no_evidence, "Is beta safe?").decision.verdict,
            WorldVerdictIR::Unknown
        );
    }

    #[test]
    fn world_unrelated_memory_does_not_change_decisions_and_cycles_do_not_invent_evidence() {
        let a = memory(&["alpha is active.", "If alpha is active, then beta is safe."]);
        let b = a.prepare("unrelated is ready.", 3).unwrap().memory;
        assert_eq!(
            reason(&a, "Is beta safe?").decision,
            reason(&b, "Is beta safe?").decision
        );
        let cycle = memory(&[
            "If alpha is active, then beta is ready.",
            "If beta is ready, then alpha is active.",
        ]);
        assert_eq!(
            reason(&cycle, "Is alpha active?").decision.verdict,
            WorldVerdictIR::Unknown
        );
    }

    #[test]
    fn world_public_negative_unknown_conflict_and_hypothetical_routes_stay_grounded() {
        for (id, texts, verdict) in [
            (
                "WORLD-UNKNOWN",
                vec!["If alpha is active, then beta is safe.", "Is beta safe?"],
                WorldVerdictIR::Unknown,
            ),
            (
                "WORLD-CONFLICT",
                vec![
                    "alpha is active.",
                    "alpha is not active.",
                    "Is alpha active?",
                ],
                WorldVerdictIR::Conflict,
            ),
            (
                "WORLD-CORRECTION",
                vec![
                    "alpha is active.",
                    "Actually, alpha is not active.",
                    "Is alpha active?",
                ],
                WorldVerdictIR::Refuted,
            ),
            (
                "WORLD-HYPOTHETICAL",
                vec![
                    "alpha is not active.",
                    "If alpha is active, then beta is safe.",
                    "Suppose alpha is active. Is beta safe?",
                ],
                WorldVerdictIR::Supported,
            ),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            for (index, text) in texts.iter().enumerate() {
                let r = request(id, index as u64 + 1, text, LanguageCodeIR::English);
                let response = api
                    .process_conversation_turn(&r)
                    .unwrap_or_else(|e| panic!("{id}:{text}: {e:?}"));
                assert!(response.validate_against(&r));
                assert!(response.grounded_response.is_none());
                assert!(response
                    .conversation_state
                    .action_state_ledger
                    .records
                    .is_empty());
                if index + 1 == texts.len() {
                    let world = response
                        .discourse_answer
                        .as_ref()
                        .unwrap()
                        .world_reasoning
                        .as_ref()
                        .unwrap();
                    assert_eq!(world.decision.verdict, verdict);
                    assert_eq!(
                        response.natural_realization.response_act,
                        crate::NaturalResponseActIR::DiscourseAnswer
                    );
                    println!("{id} => {}", response.output.text);
                }
            }
        }
    }

    #[test]
    fn world_question_answer_loop_binds_yes_to_missing_premise_not_the_final_goal() {
        for (language, turns) in [
            (
                LanguageCodeIR::English,
                vec![
                    "If alpha is active, then beta is safe.",
                    "Is beta safe?",
                    "Yes.",
                ],
            ),
            (
                LanguageCodeIR::Korean,
                vec![
                    "alpha가 가동 상태이면 beta는 안전 상태다.",
                    "beta는 안전 상태인가?",
                    "응",
                ],
            ),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            for (index, text) in turns.iter().enumerate() {
                let input = request("WORLD-ELICIT", index as u64 + 1, text, language);
                let response = api
                    .process_conversation_turn(&input)
                    .unwrap_or_else(|e| panic!("{text}: {e:?}"));
                assert!(response.validate_against(&input));
                println!("WORLD_ELICIT {text} => {}", response.output.text);
                if index == 2 {
                    let world = response
                        .discourse_answer
                        .as_ref()
                        .unwrap()
                        .world_reasoning
                        .as_ref()
                        .unwrap();
                    assert_eq!(world.decision.verdict, WorldVerdictIR::Supported);
                    assert_eq!(world.memory.premises[0].atom.entity, "alpha");
                    assert!(world.memory.premises[0].answer_binding.is_some());
                    assert!(response
                        .conversation_state
                        .action_state_ledger
                        .records
                        .is_empty());
                    let mut tampered = world.clone();
                    tampered.memory.premises[0]
                        .answer_binding
                        .as_mut()
                        .unwrap()
                        .requested_atom
                        .entity = "beta".into();
                    assert!(!tampered.validate());
                }
            }
        }
    }

    #[test]
    fn world_reply_scope_persistence_and_korean_conjunction_are_explicit() {
        let m = memory(&[
            "alpha는 가동 상태다.",
            "beta는 준비 상태다.",
            "alpha는 가동 상태이고 beta는 준비 상태이면 gamma는 안전 상태다.",
        ]);
        assert_eq!(
            reason(&m, "gamma는 안전 상태인가?").decision.verdict,
            WorldVerdictIR::Supported
        );
        let initial = memory(&["If alpha is active, then beta is safe."]);
        let pending = initial.prepare("Is beta safe?", 2).unwrap().memory;
        let no = pending.prepare("No.", 3).unwrap();
        assert_eq!(no.memory.premises[0].atom.entity, "alpha");
        assert!(!no.memory.premises[0].value);
        let no_result = deliberate_world(&no.memory, &no.query.unwrap()).unwrap();
        assert_eq!(no_result.decision.verdict, WorldVerdictIR::Unknown);
        assert!(no_result.decision.question.is_none());
        assert!(reason(&DialogueWorldIR::default(), "Is alpha active?")
            .decision
            .question
            .is_none());
        let yes = pending.prepare("Yes.", 3).unwrap().memory;
        let restored: DialogueWorldIR =
            serde_json::from_str(&serde_json::to_string(&yes).unwrap()).unwrap();
        assert!(restored.validate(3));
        assert_eq!(
            reason(&restored, "Is beta safe?").decision.verdict,
            WorldVerdictIR::Supported
        );
        let other_topic = pending
            .prepare("Let's discuss another subject.", 3)
            .unwrap()
            .memory;
        assert!(!other_topic.prepare("Yes.", 4).unwrap().recognized);
        let hypothetical = initial
            .prepare("Suppose delta is active. Is beta safe?", 2)
            .unwrap()
            .memory;
        let reply = hypothetical.prepare("Yes.", 3).unwrap();
        assert!(!reply.recognized);
        assert!(reply.memory.premises.is_empty());
    }
}
