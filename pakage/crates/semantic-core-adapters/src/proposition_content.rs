//! Typed, source-bound content slots for dialogue propositions. Unlike the
//! action planner, this compiler can retain a causal connective even when its
//! predicates have no executable operator. All content remains attributed data.

use crate::compositional_semantics::CompositionalSemanticAnalyzer;
use crate::semantic_roles::SemanticRoleKindIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContentSlotIR {
    Cause,
    Agent,
    Theme,
    Property,
    Definition,
    Summary,
    Manner,
    Recipient,
    Source,
    Location,
    Time,
    Duration,
    Intention,
    Condition,
}

/// Dialogue preferences are attributed constraints, never execution grants or
/// inferred emotional diagnoses. Surface forms are evidence, not mode identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InteractionModeIR {
    Acknowledgement,
    Explanation,
    Listening,
    Conversation,
    Concise,
    Advice,
    Summary,
    Execution,
}

/// Surface mood and communicative force are independent. A conventional
/// addressee-directed request may have interrogative syntax without asking for
/// a fact. This is source evidence, not permission for external execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InteractionMoodIR {
    Desiderative,
    Directive,
    InterrogativeRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResponseMannerIR {
    Concise,
    Detailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionPreferenceIR {
    /// How to express content, never a replacement for that content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_manner: Option<ResponseMannerIR>,
    pub desired: InteractionModeIR,
    pub mood: InteractionMoodIR,
    pub excluded: Vec<InteractionModeIR>,
    pub desired_surface: String,
    pub source_sha256: String,
    pub complete_clause_coverage: bool,
}

/// The part of a stored response preference that the current question asks to
/// surface. It is selected before wording, so generation need not quote the
/// entire source proposition to remain grounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InteractionPreferenceAnswerFocusIR {
    ModeChoice {
        desired: InteractionModeIR,
        rejected: InteractionModeIR,
    },
    ResponseLength,
}

impl InteractionPreferenceIR {
    pub fn owns_response(&self) -> bool {
        self.complete_clause_coverage
            && matches!(
                self.desired,
                InteractionModeIR::Listening
                    | InteractionModeIR::Acknowledgement
                    | InteractionModeIR::Conversation
                    | InteractionModeIR::Concise
            )
            && !self.excluded.contains(&self.desired)
    }
}

fn interaction_mode(text: &str) -> Option<InteractionModeIR> {
    let modes = interaction_modes(text);
    modes
        .iter()
        .copied()
        .find(|m| *m != InteractionModeIR::Concise)
        .or_else(|| modes.first().copied())
}

fn interaction_modes(text: &str) -> Vec<InteractionModeIR> {
    let complement = crate::modality::korean_desiderative_content(text);
    let text = complement.as_deref().unwrap_or(text);
    let words = text.split_whitespace().collect::<Vec<_>>();
    let mut modes = Vec::new();
    // Select the most specific communicative predicate, not the first noun.
    for (mode, roots) in [
        (InteractionModeIR::Acknowledgement, &["acknowledge"][..]),
        (
            InteractionModeIR::Explanation,
            &["설명", "explanation", "explain"][..],
        ),
        (
            InteractionModeIR::Listening,
            &["듣", "들어주", "들어줘", "들어줄", "listen"][..],
        ),
        (
            InteractionModeIR::Concise,
            &["짧", "간단", "brief", "short"][..],
        ),
        (
            InteractionModeIR::Conversation,
            &["얘기", "이야기", "대화", "chat", "talk"][..],
        ),
        (
            InteractionModeIR::Advice,
            &["조언", "해결책", "해법", "advice", "solution", "solve"][..],
        ),
        (
            InteractionModeIR::Summary,
            &["정리", "요약", "summari", "summariz"][..],
        ),
        (
            InteractionModeIR::Execution,
            &["실행", "수정", "삭제", "execute", "delete", "modify"][..],
        ),
    ] {
        if words.iter().any(|w| roots.iter().any(|r| w.starts_with(r))) {
            modes.push(mode);
        }
    }
    modes
}

fn interaction_request_mood(clause: &str) -> bool {
    let clause = clause.trim().trim_end_matches(['?', '!', '.']);
    // Benefactive/request endings address the listener; bare ability and
    // self-preference questions remain questions. Embedded reports do not pass.
    let korean = ["줄래", "줄래요", "주시겠어", "주시겠어요", "주시겠습니까"]
        .iter()
        .any(|ending| clause.ends_with(ending));
    let english = ["can you ", "could you ", "would you ", "will you "]
        .iter()
        .any(|prefix| clause.starts_with(prefix));
    // In an explicit English addressee request, a following wh-clause is
    // the requested content ("explain why ..."), not the matrix question.
    english
        || (korean
            && !clause
                .split_whitespace()
                .any(|w| matches!(w, "누가" | "누구" | "왜" | "who" | "why" | "whether")))
}

fn interaction_imperative(clause: &str) -> bool {
    let clause = clause.strip_prefix("just ").unwrap_or(clause);
    // Coordinated intransitive dialogue verbs, not any clause containing chat.
    clause.split(" and ").all(|part| {
        matches!(
            part.split_whitespace().next(),
            Some("listen" | "chat" | "talk" | "stay" | "acknowledge")
        ) && interaction_modes(part).iter().all(|m| {
            matches!(
                m,
                InteractionModeIR::Listening
                    | InteractionModeIR::Conversation
                    | InteractionModeIR::Acknowledgement
            )
        })
    })
}

pub(crate) fn interaction_preference(source: &str) -> Option<InteractionPreferenceIR> {
    if source.chars().count() > 1024 || source.contains(['"', '“', '”', '`']) {
        return None;
    }
    let lower = source.to_lowercase();
    let last_clause = lower.split(['.', ',', ';', '\n']).next_back()?.trim();
    if (crate::conversation_contract::is_interrogative(source) || source.contains('?'))
        && (!interaction_request_mood(last_clause)
            || lower.trim_end_matches(['?', '!', ' ']).contains('?'))
    {
        return None;
    }
    if lower.starts_with("if ")
        || lower.contains(" if ")
        || lower.contains("만약")
        || lower.contains("라고 말")
        || lower.contains("said ")
        || (crate::modality::korean_desiderative_clause(&lower)
            && !crate::modality::ModalSemanticAnalyzer
                .analyze(&lower)
                .conditionals
                .is_empty())
    {
        return None;
    }
    let first = lower.split_whitespace().next()?;
    if ["은", "는", "가", "이"].iter().any(|p| first.ends_with(p))
        && !first.ends_with("라는")
        && !first.ends_with("다는")
        && interaction_mode(first).is_none()
        && !matches!(
            first,
            "나는" | "저는" | "내가" | "제가" | "오늘은" | "지금은" | "이번에는"
        )
    {
        return None;
    }
    let mut desired = None;
    let mut response_manner = None;
    let mut excluded = Vec::new();
    let mut complete = true;
    for raw in lower
        .split(['.', ',', ';', '\n'])
        .filter(|s| !s.trim().is_empty())
    {
        let mut clause = raw.trim().trim_end_matches(['!', '?']);
        for prefix in ["그냥 ", "그리고 ", "but ", "and "] {
            clause = clause.strip_prefix(prefix).unwrap_or(clause);
        }
        if let Some((less, preferred)) = clause
            .split_once("보다 ")
            .or_else(|| clause.split_once("보다는 "))
            .or_else(|| clause.split_once("말고"))
        {
            if let Some(mode) = interaction_mode(less) {
                excluded.push(mode);
            } else {
                complete = false;
            }
            clause = preferred.trim();
            if clause.is_empty() {
                continue;
            }
        }
        let negative = clause.contains("아니고")
            || clause.contains("아니야")
            || clause.contains("필요 없")
            || clause.contains("원하지 않")
            || clause
                .split_whitespace()
                .any(|w| matches!(w, "not" | "don't" | "안" | "못"));
        let Some(mode) = interaction_mode(clause) else {
            complete = false;
            continue;
        };
        // Finding one dialogue predicate cannot certify a coordinated clause.
        // Require every subsequent English conjunct to be a dialogue imperative;
        // an unknown predicate must remain available to the general pipeline.
        // Korean conjunctive -고 is likewise not whole-clause coverage, except
        // the desiderative complement -고 싶다. This conservative bound may
        // reject valid mixed conversation, but never hides an unparsed request.
        if clause
            .split(" and ")
            .skip(1)
            .any(|part| !interaction_imperative(part.trim()))
            || (!negative
                && clause.split_whitespace().any(|w| w.ends_with('고'))
                && !clause.contains("싶"))
        {
            complete = false;
        }
        if interaction_modes(clause).contains(&InteractionModeIR::Execution)
            && mode != InteractionModeIR::Execution
        {
            let structure = CompositionalSemanticAnalyzer.analyze(clause);
            let operations = structure
                .frames
                .iter()
                .filter(|f| {
                    matches!(
                        f.intent_hint,
                        dockable_semantic_core::PlanIntentIR::Execute
                            | dockable_semantic_core::PlanIntentIR::Repair
                    )
                })
                .collect::<Vec<_>>();
            if operations.is_empty()
                || operations.iter().any(|f| {
                    !structure
                        .clause_graph
                        .node_for_frame(&f.frame_id)
                        .is_some_and(|n| {
                            n.function == crate::clause_graph::ClauseFunctionIR::ContentComplement
                        })
                })
            {
                complete = false;
            }
        }
        if negative {
            excluded.push(mode);
            continue;
        }
        if interaction_modes(clause).contains(&InteractionModeIR::Concise) {
            response_manner = Some(ResponseMannerIR::Concise);
        }
        let wish = clause.contains("싶")
            || crate::modality::korean_desiderative_clause(clause)
            || clause.contains("좋겠")
            || clause.contains("원하는 건")
            || clause.contains("원하는 것은")
            || clause.starts_with("i want ")
            || clause.starts_with("i'd like ")
            || clause.starts_with("i could use ")
            || clause.starts_with("i would like ");
        let request_question = interaction_request_mood(clause);
        let proposal = clause.ends_with('자')
            || clause
                .split_whitespace()
                .last()
                .is_some_and(crate::compositional_semantics::korean_nominal_request_tail)
            || clause.ends_with("줘")
            || clause.ends_with("주세요")
            || clause.starts_with("let's ")
            || clause.starts_with("please ")
            || interaction_imperative(clause);
        if !wish && !proposal && !request_question {
            complete = false;
            continue;
        }
        if desired.as_ref().is_some_and(|(prior, _, _)| *prior != mode) {
            return None;
        }
        let mood = if request_question {
            InteractionMoodIR::InterrogativeRequest
        } else if wish {
            InteractionMoodIR::Desiderative
        } else {
            InteractionMoodIR::Directive
        };
        desired = Some((mode, clause.to_string(), mood));
    }
    let (desired, desired_surface, mood) = desired?;
    excluded.sort();
    excluded.dedup();
    if excluded.contains(&desired) {
        return None;
    }
    Some(InteractionPreferenceIR {
        response_manner: if excluded.contains(&InteractionModeIR::Concise) {
            None
        } else {
            response_manner
        },
        desired,
        mood,
        excluded,
        desired_surface,
        source_sha256: format!("{:x}", Sha256::digest(source.as_bytes())),
        complete_clause_coverage: complete,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentBindingIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    pub slot: ContentSlotIR,
    pub value: String,
    pub predicate: Option<String>,
    pub grammar_evidence: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionContentIR {
    pub source_sha256: String,
    pub bindings: Vec<ContentBindingIR>,
    #[serde(default)]
    pub events: Vec<DescribedEventIR>,
    /// Earlier attributed inputs used to bind omitted roles. Raw source is retained.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_sources: Vec<EventSourceIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_preference: Option<InteractionPreferenceIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSourceIR {
    pub belief_id: String,
    pub source_actor: String,
    pub source_proposition: String,
}

/// An attributed event description, not a promoted executable concept. Lexical
/// alternatives identify a predicate mention; role bindings identify its arguments.
/// No ownership transfer, causal consequence or action grant follows from a label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescribedEventIR {
    #[serde(default)]
    pub kind: DescriptionKindIR,
    pub event_id: String,
    pub lexical_entry_ids: Vec<String>,
    pub predicate_surface: String,
    pub negated: bool,
    pub roles: std::collections::BTreeMap<ContentSlotIR, String>,
    /// Query-only discourse scope. A named topic selects existing role values;
    /// it does not invent a Theme binding or assert a new event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_constraint: Option<String>,
    /// Query-only type restriction supplied by a WH/reference, never a
    /// promoted assertion that an unknown bearer belongs to that type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_kind_constraint: Option<crate::lexical_knowledge_pack::NominalReferentKindIR>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DescriptionKindIR {
    #[default]
    Event,
    /// Theme is the state bearer, not an actor or an executable operation.
    State,
}

/// A replayable role correspondence, not a newly asserted event. The original
/// source event and lexical identity stay intact in memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventPerspectiveProjectionIR {
    pub relation_id: String,
    pub source_perspective: String,
    pub target_perspective: String,
}

#[derive(Deserialize)]
struct EventPerspectiveKnowledge {
    relations: Vec<EventRoleRelation>,
}

#[derive(Deserialize)]
struct EventRoleRelation {
    relation_id: String,
    required_participants: Vec<String>,
    perspectives: Vec<EventRolePerspective>,
}

#[derive(Deserialize)]
struct EventRolePerspective {
    perspective_id: String,
    lexical_entry_ids: Vec<String>,
    roles: std::collections::BTreeMap<ContentSlotIR, String>,
}

fn event_perspective_knowledge() -> &'static EventPerspectiveKnowledge {
    static KNOWLEDGE: std::sync::OnceLock<EventPerspectiveKnowledge> = std::sync::OnceLock::new();
    KNOWLEDGE.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../data/lexical-knowledge/event-role-perspectives.json"
        ))
        .expect("checked supplied role-perspective knowledge")
    })
}

impl EventRoleRelation {
    fn perspective(&self, event: &DescribedEventIR) -> Option<&EventRolePerspective> {
        let mut matches = self.perspectives.iter().filter(|p| {
            p.lexical_entry_ids
                .iter()
                .any(|id| event.lexical_entry_ids.contains(id))
        });
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
    }
}

impl EventPerspectiveProjectionIR {
    fn apply(&self, event: &DescribedEventIR) -> Option<DescribedEventIR> {
        // Absence of a loan is not a loan participant. No inverse consequence
        // is inferred from denial, missing participants or unresolved reference.
        if event.kind != DescriptionKindIR::Event || event.negated || event.has_references() {
            return None;
        }
        let relation = event_perspective_knowledge()
            .relations
            .iter()
            .find(|r| r.relation_id == self.relation_id)?;
        let source = relation.perspective(event)?;
        if source.perspective_id != self.source_perspective
            || self.source_perspective == self.target_perspective
        {
            return None;
        }
        let target = relation
            .perspectives
            .iter()
            .find(|p| p.perspective_id == self.target_perspective)?;
        let participants = event
            .roles
            .iter()
            .map(|(slot, value)| Some((source.roles.get(slot)?.as_str(), value.clone())))
            .collect::<Option<std::collections::BTreeMap<_, _>>>()?;
        if participants.len() != event.roles.len()
            || relation
                .required_participants
                .iter()
                .any(|p| !participants.contains_key(p.as_str()))
        {
            return None;
        }
        let mut view = event.clone();
        view.roles = target
            .roles
            .iter()
            .filter_map(|(slot, p)| {
                participants
                    .get(p.as_str())
                    .map(|value| (*slot, value.clone()))
            })
            .collect();
        if view.roles.len() != event.roles.len() {
            return None;
        }
        view.lexical_entry_ids = target.lexical_entry_ids.clone();
        Some(view)
    }
}

impl DescribedEventIR {
    pub(crate) fn has_unbound_participant(&self) -> bool {
        self.roles.values().any(|v| {
            speaker_or_addressee_reference(v)
                || !crate::typed_coreference::resolve_typed_coreference(&[], 0, v)
                    .ambiguous_surfaces
                    .is_empty()
                || !crate::reference_resolution_graph::scan_reference_mentions(v).is_empty()
        })
    }
    pub(crate) fn matches_topic(&self, question: &Self) -> bool {
        question.topic_constraint.as_ref().is_none_or(|topic| {
            self.roles
                .values()
                .any(|value| entity_key(value) == entity_key(topic))
        })
    }

    pub(crate) fn has_references(&self) -> bool {
        self.roles.values().any(|v| reference_kind(v).is_some())
    }

    pub fn bindings(&self) -> Vec<ContentBindingIR> {
        let mut bindings = self
            .roles
            .iter()
            .map(|(slot, value)| ContentBindingIR {
                event_id: Some(self.event_id.clone()),
                slot: *slot,
                value: value.clone(),
                predicate: None,
                grammar_evidence: format!("DESCRIBED_EVENT_ROLE:{slot:?}:{}", self.event_id),
            })
            .collect::<Vec<_>>();
        if self.kind == DescriptionKindIR::State && !self.lexical_entry_ids.is_empty() {
            bindings.push(ContentBindingIR {
                event_id: Some(self.event_id.clone()),
                slot: ContentSlotIR::Property,
                value: self.predicate_surface.clone(),
                predicate: None,
                grammar_evidence: format!("DESCRIBED_STATE_PROPERTY:{}", self.event_id),
            });
        }
        bindings
    }

    pub fn matches(&self, question: &Self, requested: ContentSlotIR) -> bool {
        self.query_view(question)
            .is_some_and(|(view, _)| view.matches_direct(question, requested))
    }

    pub(crate) fn source_role_for_query(
        &self,
        question: &Self,
        role: ContentSlotIR,
    ) -> Option<ContentSlotIR> {
        let (_, proof) = self.query_view(question)?;
        let Some(proof) = proof else {
            return Some(role);
        };
        let relation = event_perspective_knowledge()
            .relations
            .iter()
            .find(|r| r.relation_id == proof.relation_id)?;
        let source = relation
            .perspectives
            .iter()
            .find(|p| p.perspective_id == proof.source_perspective)?;
        let target = relation
            .perspectives
            .iter()
            .find(|p| p.perspective_id == proof.target_perspective)?;
        let participant = target.roles.get(&role)?;
        source
            .roles
            .iter()
            .find_map(|(slot, p)| (p == participant).then_some(*slot))
    }

    pub(crate) fn query_view(
        &self,
        question: &Self,
    ) -> Option<(Self, Option<EventPerspectiveProjectionIR>)> {
        if self.kind != question.kind {
            return None;
        }
        for relation in &event_perspective_knowledge().relations {
            if let (Some(source), Some(target)) =
                (relation.perspective(self), relation.perspective(question))
            {
                if source.perspective_id != target.perspective_id {
                    let proof = EventPerspectiveProjectionIR {
                        relation_id: relation.relation_id.clone(),
                        source_perspective: source.perspective_id.clone(),
                        target_perspective: target.perspective_id.clone(),
                    };
                    return Some((proof.apply(self)?, Some(proof)));
                }
            }
        }
        Some((self.clone(), None))
    }

    fn matches_direct(&self, question: &Self, requested: ContentSlotIR) -> bool {
        self.kind == question.kind
            && question.bearer_kind_constraint.is_none_or(|kind| {
                self.kind == DescriptionKindIR::State
                    && self
                        .roles
                        .get(&ContentSlotIR::Theme)
                        .and_then(|value| nominal_bearer_evidence(value, kind))
                        .is_some()
            })
            && (requested == ContentSlotIR::Property || self.negated == question.negated)
            && self.matches_topic(question)
            && (question.lexical_entry_ids.is_empty()
                || question
                    .lexical_entry_ids
                    .iter()
                    .any(|id| self.lexical_entry_ids.contains(id)))
            && question
                .roles
                .iter()
                .filter(|(slot, _)| **slot != requested)
                .all(|(slot, value)| {
                    self.roles
                        .get(slot)
                        .is_some_and(|actual| entity_key(actual) == entity_key(value))
                })
    }
}

/// Existing speaker/addressee vocabulary shared with attributed realization.
/// A lexical pronoun is not a resolved participant identity.
pub(crate) fn speaker_or_addressee_reference(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "i" | "you" | "we" | "나" | "저" | "내" | "제" | "너" | "우리"
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventReferenceContextIR {
    pub belief_id: String,
    pub source_actor: String,
    pub source_proposition: String,
    pub event_id: String,
    pub focused_slot: Option<ContentSlotIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_sources: Vec<EventSourceIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRoleReferenceIR {
    pub query_slot: ContentSlotIR,
    pub mention: String,
    pub antecedent_slot: ContentSlotIR,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nominal_type_evidence: Option<crate::lexical_knowledge_pack::NominalReferentEvidenceIR>,
}

enum EventReferenceKind<'a> {
    Theme,
    Person,
    Place,
    Named(&'a str),
}

fn nominal_bearer_evidence(
    value: &str,
    required: crate::lexical_knowledge_pack::NominalReferentKindIR,
) -> Option<crate::lexical_knowledge_pack::NominalReferentEvidenceIR> {
    let normalized = entity_key(value);
    let nominal = ["the ", "a ", "an "]
        .iter()
        .find_map(|article| normalized.strip_prefix(article))
        .unwrap_or(&normalized);
    crate::lexical_knowledge_pack::builtin_pack().nominal_referent_evidence_for(nominal, required)
}

impl EventReferenceKind<'_> {
    fn state_bearer_evidence(
        &self,
        event: &DescribedEventIR,
        slot: ContentSlotIR,
        value: &str,
    ) -> Option<crate::lexical_knowledge_pack::NominalReferentEvidenceIR> {
        use crate::lexical_knowledge_pack::NominalReferentKindIR;
        if event.kind != DescriptionKindIR::State || slot != ContentSlotIR::Theme {
            return None;
        }
        let required = match self {
            Self::Person => NominalReferentKindIR::Person,
            Self::Place => NominalReferentKindIR::Place,
            _ => return None,
        };
        // Articles do not change the bearer. Do not guess the head of an
        // arbitrary compound or borrow a type from a noun inside its modifier.
        let evidence = nominal_bearer_evidence(value, required)?;
        (evidence.kind == required).then_some(evidence)
    }
}

fn reference_kind(value: &str) -> Option<EventReferenceKind<'_>> {
    if let Some(base) = contracted_pronoun_topic(value) {
        return match lexical_reference_kind(&base) {
            Some(EventReferenceKind::Theme) => Some(EventReferenceKind::Theme),
            Some(EventReferenceKind::Place) => Some(EventReferenceKind::Place),
            _ => None,
        };
    }
    lexical_reference_kind(value)
}

fn lexical_reference_kind(value: &str) -> Option<EventReferenceKind<'_>> {
    Some(match value {
        "it" | "that" | "this" | "그것" | "그거" | "이것" | "이거" | "그 물건" | "that object" => {
            EventReferenceKind::Theme
        }
        "그 사람" | "그분" | "that person" | "he" | "she" | "him" | "her" | "그" | "그녀" => {
            EventReferenceKind::Person
        }
        "there" | "그곳" | "거기" | "that place" => EventReferenceKind::Place,
        other => {
            let name = other
                .strip_prefix("그 ")
                .or_else(|| other.strip_prefix("that "))?;
            if name.is_empty() {
                return None;
            }
            EventReferenceKind::Named(name)
        }
    })
}

/// Pronoun + topic particle contraction: a known vowel-final reference word
/// may carry final ㄴ instead of the separate -는 syllable. No noun/sentence
/// inventory is inferred from the spelling alone. Reuse the controlled
/// reference lexicon; general vocabulary packs need not duplicate function words.
pub(crate) fn contracted_pronoun_topic(word: &str) -> Option<String> {
    let (offset, last) = word.char_indices().next_back()?;
    let syllable = (last as u32).checked_sub(0xac00)?;
    if syllable >= 11172 || syllable % 28 != 4 {
        return None;
    }
    let base = format!("{}{}", &word[..offset], char::from_u32(last as u32 - 4)?);
    matches!(
        lexical_reference_kind(&base),
        Some(EventReferenceKind::Theme | EventReferenceKind::Place)
    )
    .then_some(base)
}

impl EventReferenceContextIR {
    fn event(&self) -> Option<DescribedEventIR> {
        if self.belief_id.is_empty() || self.source_actor.is_empty() {
            return None;
        }
        PropositionContentIR::compile_contextual(&self.source_proposition, &self.context_sources)?
            .events
            .into_iter()
            .find(|e| e.event_id == self.event_id)
    }

    /// Reconstruct role bindings from attributed source data. No output text is
    /// used as an antecedent and no arbitrary new entity is inserted.
    pub(crate) fn resolve(
        &self,
        query: &DescribedEventIR,
    ) -> Option<(DescribedEventIR, Vec<EventRoleReferenceIR>)> {
        self.resolve_selected(query, None)
    }

    fn resolve_selected(
        &self,
        query: &DescribedEventIR,
        selection: Option<(ContentSlotIR, &str)>,
    ) -> Option<(DescribedEventIR, Vec<EventRoleReferenceIR>)> {
        let event = self.event()?;
        if event.has_references() {
            return None;
        }
        let mut resolved = query.clone();
        let mut bindings = Vec::new();
        for (query_slot, mention) in &query.roles {
            let Some(kind) = reference_kind(mention) else {
                continue;
            };
            let mut candidates = event
                .roles
                .iter()
                .filter(|(slot, value)| match kind {
                    EventReferenceKind::Theme => **slot == ContentSlotIR::Theme,
                    EventReferenceKind::Place => {
                        **slot == ContentSlotIR::Location
                            || kind.state_bearer_evidence(&event, **slot, value).is_some()
                    }
                    EventReferenceKind::Person => {
                        matches!(
                            slot,
                            ContentSlotIR::Agent | ContentSlotIR::Recipient | ContentSlotIR::Source
                        ) || kind.state_bearer_evidence(&event, **slot, value).is_some()
                    }
                    EventReferenceKind::Named(name) => entity_key(value) == entity_key(name),
                })
                .collect::<Vec<_>>();
            if matches!(kind, EventReferenceKind::Person) {
                if let Some(focused) = self
                    .focused_slot
                    .filter(|s| candidates.iter().any(|(slot, _)| **slot == *s))
                {
                    candidates.retain(|(slot, _)| **slot == focused);
                }
            }
            if let Some((selected_slot, selected_value)) = selection {
                if selected_slot == *query_slot {
                    candidates.retain(|(_, value)| entity_key(value) == entity_key(selected_value));
                }
            }
            let (antecedent_slot, value) = *candidates.first()?;
            if candidates
                .iter()
                .any(|(_, v)| entity_key(v) != entity_key(value))
            {
                return None;
            }
            resolved.roles.insert(*query_slot, value.clone());
            bindings.push(EventRoleReferenceIR {
                query_slot: *query_slot,
                mention: mention.clone(),
                antecedent_slot: *antecedent_slot,
                value: value.clone(),
                nominal_type_evidence: kind.state_bearer_evidence(&event, *antecedent_slot, value),
            });
        }
        if query.kind == DescriptionKindIR::State
            && query.lexical_entry_ids.is_empty()
            && !bindings.is_empty()
        {
            resolved.topic_constraint = resolved.roles.get(&ContentSlotIR::Theme).cloned();
        }
        Some((resolved, bindings))
    }
}

/// A missing reference binding, not a cached answer. Choices are reconstructed
/// from source events, and their question surfaces must parse back to the
/// selected semantic query. Only one reference dimension is selected at a time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventReferenceGapIR {
    pub source_question: String,
    pub slot: ContentSlotIR,
    pub contexts: Vec<EventReferenceContextIR>,
}

impl EventReferenceGapIR {
    pub(crate) fn refers_to_person(&self) -> bool {
        described_event(&self.source_question, true)
            .and_then(|q| q.roles.get(&self.slot).cloned())
            .is_some_and(|v| matches!(reference_kind(&v), Some(EventReferenceKind::Person)))
    }
    pub(crate) fn from_contexts(
        source: &str,
        contexts: &[EventReferenceContextIR],
    ) -> Option<Self> {
        let query = described_event(source, true)?;
        query
            .roles
            .iter()
            .filter(|(_, value)| reference_kind(value).is_some())
            .find_map(|(slot, _)| {
                let gap = Self {
                    source_question: source.into(),
                    slot: *slot,
                    contexts: contexts.to_vec(),
                };
                gap.choices().map(|_| gap)
            })
    }

    pub(crate) fn choices(&self) -> Option<Vec<(String, String)>> {
        if self.contexts.is_empty() || self.contexts.len() > 8 {
            return None;
        }
        let query = described_event(&self.source_question, true)?;
        reference_kind(query.roles.get(&self.slot)?)?;
        let mut choices =
            std::collections::BTreeMap::<String, (String, String, DescribedEventIR)>::new();
        for context in &self.contexts {
            let event = context.event()?;
            for value in event.roles.values() {
                let Some((resolved, bindings)) =
                    context.resolve_selected(&query, Some((self.slot, value)))
                else {
                    continue;
                };
                let Some(question) =
                    selected_reference_question(&self.source_question, &resolved, &bindings)
                else {
                    continue;
                };
                let key = entity_key(value);
                if let Some((_, _, previous)) = choices.get(&key) {
                    if previous.roles != resolved.roles {
                        return None;
                    }
                } else {
                    choices.insert(key, (value.clone(), question, resolved));
                }
            }
        }
        (2..=8)
            .contains(&choices.len())
            .then(|| choices.into_values().map(|(v, q, _)| (v, q)).collect())
    }

    pub(crate) fn live_in(&self, state: &crate::conversation::ConversationStateIR) -> bool {
        self.choices().is_some()
            && self.contexts.iter().all(|c| {
                state.epistemic_ledger.records.iter().any(|r| {
                    r.belief_id == c.belief_id
                        && r.source_actor == c.source_actor
                        && r.proposition_surface == c.source_proposition
                        && r.status == crate::epistemic::BeliefRecordStatusIR::Active
                        && r.signature.modal_world == crate::modality::ModalWorldIR::Actual
                        && r.content.validate_source(&r.proposition_surface)
                        && r.content.context_sources == c.context_sources
                        && r.content.events.iter().any(|e| e.event_id == c.event_id)
                })
            })
    }
}

fn selected_reference_question(
    source: &str,
    expected: &DescribedEventIR,
    bindings: &[EventRoleReferenceIR],
) -> Option<String> {
    let folded = source.to_ascii_lowercase();
    let mut replacements = Vec::new();
    for binding in bindings {
        let mention = binding.mention.to_ascii_lowercase();
        let mut spans = folded.match_indices(&mention);
        let (start, _) = spans.next()?;
        if spans.next().is_some() {
            return None;
        }
        let replacement = if contracted_pronoun_topic(&binding.mention).is_some() {
            let last = binding.value.chars().next_back()? as u32;
            if !(0xac00..=0xd7a3).contains(&last) {
                return None;
            }
            format!(
                "{}{}",
                binding.value,
                if (last - 0xac00).is_multiple_of(28) {
                    "는"
                } else {
                    "은"
                }
            )
        } else {
            binding.value.clone()
        };
        replacements.push((start, start + mention.len(), replacement));
    }
    replacements.sort_by_key(|r| r.0);
    if replacements.windows(2).any(|r| r[0].1 > r[1].0) {
        return None;
    }
    let mut selected = source.to_string();
    for (start, end, value) in replacements.into_iter().rev() {
        selected.replace_range(start..end, &value);
    }
    let actual = described_event(&selected, true)?;
    (actual.roles.iter().all(|(slot, v)| {
        expected
            .roles
            .get(slot)
            .is_some_and(|e| entity_key(e) == entity_key(v))
    }) && actual.roles.len() == expected.roles.len()
        && actual.lexical_entry_ids == expected.lexical_entry_ids
        && actual.kind == expected.kind
        && actual.negated == expected.negated
        && actual.topic_constraint == expected.topic_constraint
        && actual.bearer_kind_constraint == expected.bearer_kind_constraint)
        .then_some(selected)
}

fn entity_key(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "나" | "저" | "내" | "제" | "i" | "me" => "DIALOGUE_USER".into(),
        other => other.to_string(),
    }
}

fn explicit_korean_desire_nominal(source: &str) -> Option<String> {
    let text = source.trim().trim_end_matches(['.', '!']).trim();
    let (_, complement) = ["원하는 건 ", "원하는 것은 ", "바라는 건 ", "바라는 것은 "]
        .into_iter()
        .find_map(|marker| text.split_once(marker))?;
    for ending in ["입니다", "이에요", "예요", "이야", "야"] {
        if let Some(nominal) = complement.strip_suffix(ending).map(str::trim) {
            if !nominal.is_empty() {
                return Some(nominal.to_string());
            }
        }
    }
    None
}

fn korean_action_desire_nominal(source: &str) -> Option<String> {
    let text = source.trim().trim_end_matches(['.', '!']).trim();
    for suffix in ["고 싶어", "고 싶어요", "고 싶습니다"] {
        if let Some(stem) = text.strip_suffix(suffix).map(str::trim) {
            if !stem.is_empty() {
                return Some(format!("{stem}기"));
            }
        }
    }
    None
}

fn desired_content_with_evidence(source: &str) -> Option<(String, &'static str)> {
    if let Some(preference) = interaction_preference(source) {
        if let Some(nominal) = explicit_korean_desire_nominal(&preference.desired_surface) {
            return Some((nominal, "DESIDERATIVE_NOMINAL"));
        }
        return Some((preference.desired_surface, "DESIDERATIVE_COMPLEMENT"));
    }
    if source.contains(['"', '“', '”', '?'])
        || crate::conversation_contract::is_interrogative(source)
    {
        return None;
    }
    let text = source.trim().trim_end_matches(['.', '!']).trim();
    if text.split_whitespace().next().is_some_and(|w| {
        ["는", "은", "가", "이"].iter().any(|s| w.ends_with(s))
            && !matches!(w, "나는" | "저는" | "내가" | "제가")
    }) {
        return None;
    }
    if let Some(nominal) = korean_action_desire_nominal(text) {
        return Some((nominal, "DESIDERATIVE_NOMINAL"));
    }
    let lower = text.to_lowercase();
    for prefix in ["i want to ", "i would like to ", "i'd like to "] {
        if let Some(rest) = lower.strip_prefix(prefix).filter(|s| !s.is_empty()) {
            return Some((rest.into(), "DESIDERATIVE_COMPLEMENT"));
        }
    }
    None
}

pub(crate) fn desired_content(source: &str) -> Option<String> {
    desired_content_with_evidence(source).map(|(value, _)| value)
}

pub(crate) fn wants_conversation(source: &str) -> bool {
    if let Some(preference) = interaction_preference(source) {
        return preference.owns_response()
            && matches!(
                preference.desired,
                InteractionModeIR::Listening | InteractionModeIR::Conversation
            );
    }
    desired_content(source).is_some_and(|v| {
        !v.split_whitespace()
            .any(|w| matches!(w, "안" | "못" | "not" | "never"))
            && v.split_whitespace().any(|w| {
                let root = w.trim_end_matches("하기");
                matches!(root, "얘기" | "이야기" | "대화" | "talk" | "chat")
            })
    })
}

pub(crate) fn is_event_report(source: &str) -> bool {
    if crate::conversation_contract::is_interrogative(source) {
        return false;
    }
    if crate::modality::ModalSemanticAnalyzer
        .analyze(source)
        .root_world
        != crate::modality::ModalWorldIR::Actual
    {
        return false;
    }
    if nominal_role_revision(source).is_some() {
        return true;
    }
    let lower = source.to_lowercase();
    if ["you ", "너는 ", "she ", "he ", "they "]
        .iter()
        .any(|s| lower.starts_with(s))
    {
        return false;
    }
    let Some(event) = described_event(reported_event_surface(source), false).or_else(|| {
        if let Some((_, tail)) = source.split_once(" 아니라 ") {
            return described_event(tail, false);
        }
        let tail = source
            .strip_prefix("No, ")
            .or_else(|| source.strip_prefix("no, "))
            .or_else(|| source.strip_prefix("Actually, "))?;
        described_event(
            tail.split_once(", not ").map_or(tail, |(new, _)| new),
            false,
        )
    }) else {
        return false;
    };
    let ko = lower.chars().any(|c| ('가'..='힣').contains(&c));
    if ko {
        ["었", "았", "였", "했", "줬"]
            .iter()
            .any(|m| event.predicate_surface.contains(m))
    } else {
        event.roles.contains_key(&ContentSlotIR::Agent)
    }
}

fn predicate_entries(word: &str) -> Vec<String> {
    let lookup = crate::lexical_knowledge_pack::builtin_pack().lookup(word);
    if lookup.truncated {
        return Vec::new();
    }
    let mut ids = lookup
        .matches
        .into_iter()
        .filter(|m| m.entry.pos == "동사" && m.matched_form == word.to_lowercase())
        .map(|m| m.entry.source_entry_id)
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

/// In a relative clause such as `빌려 간 사람`, the adnominal directional
/// auxiliary keeps the principal event and its participants in focus. Both
/// words must be independently licensed by the lexical pack: an arbitrary
/// two-word sequence cannot acquire the principal predicate's identity.
fn korean_relative_directional_principal<'a>(
    words: &mut Vec<&'a str>,
    auxiliary: &str,
) -> Option<(&'a str, Vec<String>)> {
    let auxiliary_lookup = crate::lexical_knowledge_pack::builtin_pack().lookup(auxiliary);
    let directional_adnominal = !auxiliary_lookup.truncated
        && auxiliary_lookup.matches.iter().any(|m| {
            m.matched_form == auxiliary
                && matches!(m.entry.lemma.as_str(), "가다" | "오다")
                && m.morphology.grammar_rule == "KO_ADNOMINAL_PAST"
        });
    if !directional_adnominal {
        return None;
    }
    let principal = words.last().copied()?;
    let principal_lookup = crate::lexical_knowledge_pack::builtin_pack().lookup(principal);
    let mut ids = principal_lookup
        .matches
        .iter()
        .filter(|m| {
            m.matched_form == principal
                && m.entry.pos == "동사"
                && matches!(
                    m.morphology.grammar_rule.as_str(),
                    "SOURCE_KOREAN_PRINCIPAL_FORM" | "KO_CONNECTIVE_CONTRACTION"
                )
        })
        .map(|m| m.entry.source_entry_id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return None;
    }
    words.pop();
    Some((principal, ids))
}

fn time_word(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "어제" | "오늘" | "내일" | "그제" | "모레" | "yesterday" | "today" | "tomorrow"
    )
}

fn role_particle(word: &str) -> Option<(ContentSlotIR, &str)> {
    for (suffix, role) in [
        ("에게서", ContentSlotIR::Source),
        ("한테서", ContentSlotIR::Source),
        ("에게", ContentSlotIR::Recipient),
        ("한테", ContentSlotIR::Recipient),
        ("께", ContentSlotIR::Recipient),
        ("에서", ContentSlotIR::Location),
        ("은", ContentSlotIR::Agent),
        ("는", ContentSlotIR::Agent),
        ("이", ContentSlotIR::Agent),
        ("가", ContentSlotIR::Agent),
        ("을", ContentSlotIR::Theme),
        ("를", ContentSlotIR::Theme),
    ] {
        if let Some(stem) = word.strip_suffix(suffix).filter(|v| !v.is_empty()) {
            return Some((role, stem));
        }
    }
    None
}

fn add_role(
    roles: &mut std::collections::BTreeMap<ContentSlotIR, String>,
    role: ContentSlotIR,
    value: String,
) -> Option<()> {
    if value.trim().is_empty() || value.chars().count() > 96 || roles.insert(role, value).is_some()
    {
        None
    } else {
        Some(())
    }
}

/// Full-consumption role grammar. Unconsumed words, duplicate roles, quoted or
/// embedded clauses fail closed. Unknown entity phrases are allowed; predicates
/// must come from the indexed dictionary, without converting definitions to code.
pub(crate) fn described_event(source: &str, question: bool) -> Option<DescribedEventIR> {
    described_event_inner(source, question, false)
}

/// Non-finite action content, not an observation. Only a caller that has already
/// parsed a proposal envelope may use this entry point. It shares role and
/// polarity grammar with reports without inventing a subject or a past event.
pub(crate) fn proposed_event(source: &str) -> Option<DescribedEventIR> {
    described_event_inner(source, false, true)
}

fn described_event_inner(source: &str, question: bool, proposal: bool) -> Option<DescribedEventIR> {
    if question {
        if let Some((topic, body)) = crate::conversation::topic_question_parts(source) {
            let mut event = described_event(body, true)?;
            event.topic_constraint = Some(topic.surface);
            return Some(event);
        }
    }
    if !question && is_event_correction_surface(source) {
        return None;
    }
    if !proposal {
        if let Some(state) = described_state(source, question) {
            return Some(state);
        }
    }
    if source.chars().count() > 1024 || source.contains(['"', '“', '”', '‘', '’', '`', '\n', ';'])
    {
        return None;
    }
    let raw = source.trim().trim_end_matches(['.', '?', '!']).trim();
    let normalized = if question {
        normalize_role_question(raw)
    } else {
        raw.to_string()
    };
    let text = normalized.as_str();
    if text.contains(['.', '?', '!'])
        || [
            " because ",
            " if ",
            " and ",
            "다고",
            "라면",
            "때문에",
            "라고",
        ]
        .iter()
        .any(|s| text.contains(s))
    {
        return None;
    }
    if !question && crate::conversation_contract::is_interrogative(source) {
        return None;
    }
    if !question && !proposal {
        if let Some(event) = passive_event_report(text, source) {
            return Some(event);
        }
    }
    if question {
        if let Some(event) = passive_role_question(text, source) {
            return Some(event);
        }
    }
    let korean = text.chars().any(|c| ('가'..='힣').contains(&c));
    let mut words = text.split_whitespace().collect::<Vec<_>>();
    if words.len() > 24 || words.is_empty() {
        return None;
    }
    let mut roles = std::collections::BTreeMap::new();
    let mut filtered = Vec::new();
    for word in words.drain(..) {
        if time_word(word) {
            add_role(&mut roles, ContentSlotIR::Time, word.into())?;
        } else {
            filtered.push(word);
        }
    }
    words = filtered;
    let mut negated = false;
    if question {
        let requested = requested_content_slot(source);
        if let Some(slot) = requested {
            words.retain(|word| {
                !matches!(
                    word.to_lowercase().as_str(),
                    "누가"
                        | "누구에게"
                        | "누구한테"
                        | "누구에게서"
                        | "누구한테서"
                        | "뭘"
                        | "무엇을"
                        | "언제"
                        | "어디서"
                        | "어디에서"
                        | "얼마나"
                )
            });
            if !korean {
                let lower = words.iter().map(|w| w.to_lowercase()).collect::<Vec<_>>();
                let n = if lower.starts_with(&["to".into(), "whom".into()])
                    || lower.starts_with(&["from".into(), "whom".into()])
                    || lower.starts_with(&["how".into(), "long".into()])
                {
                    2
                } else if lower
                    .first()
                    .is_some_and(|w| matches!(w.as_str(), "who" | "what" | "when" | "where"))
                {
                    1
                } else {
                    0
                };
                words.drain(..n);
                if words
                    .first()
                    .is_some_and(|w| matches!(w.to_lowercase().as_str(), "did" | "does" | "do"))
                {
                    words.remove(0);
                }
            }
            if words.is_empty() {
                return Some(DescribedEventIR {
                    kind: DescriptionKindIR::Event,
                    event_id: String::new(),
                    lexical_entry_ids: vec![],
                    predicate_surface: String::new(),
                    negated,
                    roles,
                    topic_constraint: None,
                    bearer_kind_constraint: None,
                });
            }
            if slot == ContentSlotIR::Recipient && words.last().is_some_and(|w| *w == "to") {
                words.pop();
            }
        }
    }
    let (predicate, ids) = if korean {
        let last = words.pop()?;
        // Long-form negation is predicate morphology, not a second event.
        let negative_stem;
        let predicate = if matches!(last, "않은" | "않는" | "않았어" | "않았다") {
            negative_stem = format!("{}다", words.pop()?.strip_suffix('지')?);
            negated = true;
            negative_stem.as_str()
        } else {
            last
        };
        // Only a local predicate negator belongs to this single-clause grammar.
        if words.last().is_some_and(|w| matches!(*w, "안" | "못")) {
            words.pop();
            negated = true;
        }
        if words.iter().any(|w| matches!(*w, "안" | "못")) {
            return None;
        }
        if ["고", "지만", "면", "자", "세요", "십시오"]
            .iter()
            .any(|s| predicate.ends_with(s))
        {
            return None;
        }
        let (predicate, ids) = korean_relative_directional_principal(&mut words, predicate)
            .map_or_else(
                || (predicate.to_string(), predicate_entries(predicate)),
                |(principal, ids)| (format!("{principal} {predicate}"), ids),
            );
        if ids.is_empty() {
            return None;
        }
        let mut phrase = Vec::new();
        for word in words {
            if let Some((role, stem)) = role_particle(word) {
                phrase.push(stem);
                add_role(&mut roles, role, phrase.join(" "))?;
                phrase.clear();
            } else if matches!(word, "시간" | "분" | "초") && !phrase.is_empty() {
                phrase.push(word);
                add_role(&mut roles, ContentSlotIR::Duration, phrase.join(" "))?;
                phrase.clear();
            } else {
                phrase.push(word);
            }
        }
        if !phrase.is_empty() {
            return None;
        }
        (predicate, ids)
    } else {
        // Explicit subject + finite verb, or an agent wh-gap + finite verb.
        // Multiword entities remain intact in prepositional/object positions.
        let index = if proposal || requested_content_slots(source).contains(&ContentSlotIR::Agent) {
            0
        } else if words
            .first()
            .is_some_and(|w| w.eq_ignore_ascii_case("that"))
        {
            2
        } else {
            1
        };
        let finite_auxiliary = !proposal
            && words
                .get(index)
                .is_some_and(|w| matches!(w.to_lowercase().as_str(), "did" | "does" | "do"));
        if finite_auxiliary {
            words.remove(index);
        }
        if words
            .get(index)
            .is_some_and(|w| w.eq_ignore_ascii_case("not"))
        {
            // A declarative lexical verb needs do-support for negation.
            // Without it, an initial auxiliary in a prohibition can be
            // mistaken for the subject ("Do not ..."). Questions already
            // consumed their fronted auxiliary; passives were parsed above.
            if !question && !proposal && !finite_auxiliary {
                return None;
            }
            words.remove(index);
            negated = true;
        }
        if words.iter().any(|w| w.eq_ignore_ascii_case("not")) {
            return None;
        }
        let predicate = *words.get(index)?;
        let ids = predicate_entries(predicate);
        if ids.is_empty() {
            return None;
        }
        if index > 0 {
            let subject = words[..index].join(" ");
            // A determiner alone cannot fill an agent NP. In particular, a
            // rejected passive must not fall back to reading its theme noun
            // as an active verb with "the" as the supposed actor.
            if matches!(subject.to_lowercase().as_str(), "the" | "a" | "an") {
                return None;
            }
            add_role(&mut roles, ContentSlotIR::Agent, subject)?;
        }
        let mut current = ContentSlotIR::Theme;
        let mut phrase = Vec::new();
        let mut pending_preposition = false;
        for word in &words[index + 1..] {
            // A bare in-situ object WH fills the same typed gap as a fronted
            // object question. It is not a literal object called "what".
            if question
                && current == ContentSlotIR::Theme
                && phrase.is_empty()
                && word.eq_ignore_ascii_case("what")
                && words.last() == Some(word)
                && requested_content_slots(source).contains(&ContentSlotIR::Theme)
            {
                continue;
            }
            let role = match word.to_lowercase().as_str() {
                "to" => Some(ContentSlotIR::Recipient),
                "from" => Some(ContentSlotIR::Source),
                "in" | "at" => Some(ContentSlotIR::Location),
                "for" => Some(ContentSlotIR::Duration),
                _ => None,
            };
            if let Some(role) = role {
                if pending_preposition && phrase.is_empty() {
                    return None;
                }
                if !phrase.is_empty() {
                    add_role(&mut roles, current, phrase.join(" "))?;
                    phrase.clear();
                }
                current = role;
                pending_preposition = true;
            } else if !(phrase.is_empty()
                && matches!(word.to_lowercase().as_str(), "the" | "a" | "an"))
            {
                phrase.push(*word);
            }
        }
        if pending_preposition && phrase.is_empty() {
            return None;
        }
        if !phrase.is_empty() {
            add_role(&mut roles, current, phrase.join(" "))?;
        }
        (predicate.to_string(), ids)
    };
    if roles.is_empty() && !question && !proposal {
        return None;
    }
    Some(DescribedEventIR {
        kind: DescriptionKindIR::Event,
        event_id: format!("DESCRIBED_EVENT_{:x}", Sha256::digest(source.as_bytes())),
        lexical_entry_ids: ids,
        predicate_surface: predicate,
        negated,
        roles,
        topic_constraint: None,
        bearer_kind_constraint: None,
    })
}

/// The event argument of an explicit, single causal construction. This only
/// selects a source span; event grammar still has to consume and validate it.
pub(crate) fn reported_event_surface(source: &str) -> &str {
    if source.contains(['"', '“', '”', ';', '\n'])
        || crate::conversation_contract::is_interrogative(source)
    {
        return source;
    }
    let lower = source.to_ascii_lowercase();
    if lower.matches(" because ").count() == 1 {
        let pos = lower.find(" because ").unwrap();
        if !source[..pos].trim().is_empty() && !source[pos + 9..].trim().is_empty() {
            return source[..pos].trim();
        }
    } else if lower.matches(" 때문에 ").count() == 1 {
        let pos = lower.find(" 때문에 ").unwrap();
        if !source[..pos].trim().is_empty() && !source[pos + " 때문에 ".len()..].trim().is_empty()
        {
            return source[pos + " 때문에 ".len()..].trim();
        }
    }
    stative_causal_spans(source).map_or(source, |(_, effect)| effect)
}

/// Lexical identity, not a substring or guessed stem. Competing verbal readings
/// prevent a stative interpretation. This grants no truth or action authority.
pub(crate) fn stative_predicate_ids(word: &str) -> Vec<String> {
    let lookup = crate::lexical_knowledge_pack::builtin_pack().lookup(word);
    if lookup.truncated {
        return vec![];
    }
    let exact = lookup.matches.iter().filter(|m| m.matched_form == word);
    if exact
        .clone()
        .any(|m| matches!(m.entry.pos.as_str(), "동사" | "보조 동사"))
    {
        return vec![];
    }
    exact
        .filter(|m| m.entry.pos == "형용사")
        .map(|m| m.entry.source_entry_id.clone())
        .collect()
}

/// English copular syntax supplies the state construction. A source Korean
/// verb may express the same property with a `be + complement` English alias;
/// Korean POS is not an English POS label. Retain all matching lexical IDs.
fn english_copular_predicate_ids(complement: &str) -> Vec<String> {
    let pack = crate::lexical_knowledge_pack::builtin_pack();
    let bare = pack.lookup(complement);
    let copular_form = format!("be {complement}");
    let copular = pack.lookup(&copular_form);
    if bare.truncated || copular.truncated {
        return vec![];
    }
    let verbal_participle = bare.matches.iter().any(|m| {
        m.matched_form == complement
            && m.entry.pos == "동사"
            && matches!(
                m.morphology.grammar_rule.as_str(),
                "EN_PAST_PARTICIPLE" | "EN_REGULAR_PAST" | "EN_IRREGULAR_PAST"
            )
    });
    bare.matches
        .iter()
        .filter(|m| m.matched_form == complement && m.entry.pos == "형용사")
        .chain(copular.matches.iter().filter(|m| {
            !verbal_participle
                && m.matched_form == copular_form
                && m.morphology.grammar_rule == "SOURCE_ENGLISH_EQUIVALENT"
                && matches!(m.entry.pos.as_str(), "형용사" | "동사" | "보조 동사")
        }))
        .map(|m| m.entry.source_entry_id.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// A source-bound state relation uses the same evidence/projection pipeline as
/// event descriptions, but has no agent or action semantics. Full consumption
/// keeps embedded clauses, missing bearers and unhandled scope out of memory.
fn state_property_head(word: &str) -> bool {
    matches!(word, "상태" | "state" | "condition")
}

fn korean_copular_head(word: &str) -> Option<&str> {
    ["입니까", "인가요", "이에요", "예요", "인가", "이야", "야"]
        .iter()
        .find_map(|ending| word.strip_suffix(ending))
}

fn korean_state_nominal(prefix: &[&str], stem: &str) -> Option<String> {
    if stem.is_empty() || matches!(stem, "누구" | "무엇" | "뭐") {
        return None;
    }
    let bearer = prefix
        .iter()
        .copied()
        .chain([stem])
        .collect::<Vec<_>>()
        .join(" ");
    let known_reference = matches!(
        lexical_reference_kind(&bearer),
        Some(EventReferenceKind::Theme | EventReferenceKind::Person | EventReferenceKind::Place)
    );
    (known_reference || prefix.iter().all(|w| w.ends_with('의'))).then_some(bearer)
}

/// The noun denotes an information category, not another entity to retrieve.
/// Compile the owner and open property directly; never rewrite to a canned
/// question and reparse it or choose a reply here.
fn nominal_state_bearer(words: &[&str], korean: bool) -> Option<String> {
    if korean {
        let (last, body) = words.split_last()?;
        if body.last() == Some(&"어떤")
            && korean_copular_head(last).is_some_and(state_property_head)
        {
            let (subject, prefix) = body[..body.len() - 1].split_last()?;
            if *subject == "누가" {
                return None;
            }
            return korean_state_nominal(prefix, subject.strip_suffix(['은', '는', '이', '가'])?);
        }
        let (head, owners) = body.split_last()?;
        if !state_property_head(head.strip_suffix(['은', '는', '이', '가']).unwrap_or(head)) {
            return None;
        }
        let interrogative = korean_copular_head(last)
            .is_some_and(|head| matches!(head, "무엇" | "뭐"))
            || crate::lexical_knowledge_pack::builtin_pack()
                .lookup(last)
                .matches
                .iter()
                .any(|m| m.matched_form == *last && m.entry.lemma == "어떻다");
        if !interrogative {
            return None;
        }
        let (owner, prefix) = owners.split_last()?;
        korean_state_nominal(prefix, owner.strip_suffix('의')?)
    } else {
        let [wh, copula, tail @ ..] = words else {
            return None;
        };
        if !matches!(wh.to_ascii_lowercase().as_str(), "what" | "how") || *copula != "is" {
            return None;
        }
        let tail = tail.strip_prefix(&["the"]).unwrap_or(tail);
        let [head, "of", bearer @ ..] = tail else {
            return None;
        };
        if !state_property_head(head)
            || bearer.is_empty()
            || bearer
                .iter()
                .any(|w| matches!(*w, "and" | "or" | "if" | "because" | "not" | "that"))
        {
            return None;
        }
        Some(bearer.join(" "))
    }
}

fn described_state(source: &str, question: bool) -> Option<DescribedEventIR> {
    if source.len() > 1024 || source.contains(['"', '“', '”', '‘', '’', '`', '\n', ';']) {
        return None;
    }
    if !question && crate::conversation_contract::is_interrogative(source) {
        return None;
    }
    let body = source.trim().trim_end_matches(['.', '?', '!']);
    if body.contains(['.', '?', '!']) {
        return None;
    }
    let mut words = body.split_whitespace().collect::<Vec<_>>();
    if words.len() < 2 || words.len() > 16 {
        return None;
    }
    let korean = body.chars().any(|c| ('가'..='힣').contains(&c));
    if let Some(bearer) = question
        .then(|| nominal_state_bearer(&words, korean))
        .flatten()
    {
        let mut roles = std::collections::BTreeMap::new();
        add_role(&mut roles, ContentSlotIR::Theme, bearer.clone())?;
        return Some(DescribedEventIR {
            kind: DescriptionKindIR::State,
            event_id: format!("STATE:{:x}", Sha256::digest(source.as_bytes())),
            lexical_entry_ids: vec![],
            predicate_surface: String::new(),
            negated: false,
            roles,
            topic_constraint: reference_kind(&bearer).is_none().then_some(bearer),
            bearer_kind_constraint: None,
        });
    }
    let mut negated = false;
    let mut property_gap = false;
    let mut bearer_kind_constraint = None;
    let (bearer, predicate, ids, gap) = if korean {
        let mut head = words.pop()?;
        let mut quoted = false;
        if question && words.last().is_some_and(|w| w.ends_with("다고")) {
            let reporting = crate::lexical_knowledge_pack::builtin_pack().lookup(head);
            if !reporting.matches.iter().any(|m| {
                m.matched_form == head
                    && matches!(m.entry.lemma.as_str(), "하다" | "말하다")
                    && m.morphology.grammar_rule == "KO_PRINCIPAL_FORM_PAST_ENDING"
            }) {
                return None;
            }
            head = words.pop()?;
            quoted = true;
        }
        let negative_lemma;
        let mut predicate_surface = head.to_string();
        let predicate = if matches!(head, "않다" | "않아" | "않아요" | "않았다" | "않았어")
        {
            let complement = words.pop()?;
            negative_lemma = format!("{}다", complement.strip_suffix('지')?);
            predicate_surface = format!("{complement} {head}");
            negated = true;
            negative_lemma.as_str()
        } else {
            head
        };
        if words.last() == Some(&"안") {
            words.pop();
            negated = true;
            predicate_surface = format!("안 {predicate_surface}");
        }
        let mut ids = stative_predicate_ids(predicate);
        if ids.is_empty() {
            return None;
        }
        let finite = crate::lexical_knowledge_pack::builtin_pack().lookup(predicate);
        let interrogative_property = finite
            .matches
            .iter()
            .any(|m| m.matched_form == predicate && m.entry.lemma == "어떻다");
        if interrogative_property {
            if !question || negated {
                return None;
            }
            property_gap = true;
            ids.clear();
        }
        if !property_gap
            && !finite.matches.iter().any(|m| {
                m.matched_form == predicate
                    && ids.contains(&m.entry.source_entry_id)
                    && match m.morphology.grammar_rule.as_str() {
                        "LEXICAL_LEMMA" => true,
                        "KO_STATIVE_QUOTATIVE" => quoted,
                        "KO_CONNECTIVE_CONTRACTION" | "KO_CONNECTIVE_POLITE" => true,
                        "SOURCE_KOREAN_PRINCIPAL_FORM" => {
                            crate::lexical_knowledge_pack::is_connective_principal_form(predicate)
                        }
                        "KO_STEM_SENTENTIAL_ENDING" | "KO_PRINCIPAL_FORM_PAST_ENDING" => !matches!(
                            m.morphology.ending.as_str(),
                            "고" | "지만" | "지" | "긴" | "기" | "는데" | "을까" | "을까요"
                        ),
                        _ => false,
                    }
            })
        {
            return None;
        }
        let subject = words.pop()?;
        if subject == "누가" {
            bearer_kind_constraint =
                Some(crate::lexical_knowledge_pack::NominalReferentKindIR::Person);
        }
        let gap = matches!(subject, "뭐가" | "무엇이" | "누가");
        let stem = if gap {
            ""
        } else if contracted_pronoun_topic(subject).is_some() {
            subject
        } else {
            ["이", "가", "은", "는"]
                .iter()
                .find_map(|s| subject.strip_suffix(s))?
        };
        let mut nominal_words = words.clone();
        nominal_words.push(stem);
        let bearer = nominal_words.join(" ");
        let known_reference = matches!(
            lexical_reference_kind(&bearer),
            Some(
                EventReferenceKind::Theme | EventReferenceKind::Person | EventReferenceKind::Place
            )
        );
        // Reuse known referential noun phrases; otherwise preceding words must
        // be genitive-marked. Do not consume arbitrary modifiers or clauses.
        if (!known_reference && words.iter().any(|w| !w.ends_with('의')))
            || (gap && !words.is_empty())
        {
            return None;
        }
        words.push(stem);
        (
            words.join(" ").trim().to_string(),
            predicate_surface,
            ids,
            gap,
        )
    } else {
        let index = words
            .iter()
            .position(|w| matches!(*w, "am" | "is" | "are" | "was" | "were"))?;
        if index == 0 {
            return None;
        }
        if question && index == 1 && words[0].eq_ignore_ascii_case("how") {
            let bearer = words[index + 1..].join(" ");
            if bearer.is_empty()
                || words[index + 1..].iter().any(|w| {
                    matches!(
                        w.to_ascii_lowercase().as_str(),
                        "if" | "because" | "and" | "or" | "not"
                    )
                })
            {
                return None;
            }
            let mut roles = std::collections::BTreeMap::new();
            add_role(&mut roles, ContentSlotIR::Theme, bearer.clone())?;
            return Some(DescribedEventIR {
                kind: DescriptionKindIR::State,
                event_id: String::new(),
                lexical_entry_ids: vec![],
                predicate_surface: String::new(),
                negated: false,
                roles,
                topic_constraint: reference_kind(&bearer).is_none().then_some(bearer),
                bearer_kind_constraint: None,
            });
        }
        if words[..index].iter().any(|w| {
            matches!(
                w.to_ascii_lowercase().as_str(),
                "if" | "when" | "because" | "that" | "and" | "or" | "not" | "said" | "says"
            )
        }) {
            return None;
        }
        let bearer = words[..index].join(" ");
        if bearer.eq_ignore_ascii_case("who") {
            bearer_kind_constraint =
                Some(crate::lexical_knowledge_pack::NominalReferentKindIR::Person);
        }
        let mut tail = &words[index + 1..];
        if tail.first() == Some(&"not") {
            negated = true;
            tail = &tail[1..];
        }
        let [predicate] = tail else {
            return None;
        };
        // A finite copula supplies the syntactic adjective constraint; an
        // unrelated verbal dictionary sense must not erase that reading.
        let folded = predicate.to_ascii_lowercase();
        let ids = english_copular_predicate_ids(&folded);
        if ids.is_empty() {
            return None;
        }
        let gap = bearer.eq_ignore_ascii_case("what") || bearer.eq_ignore_ascii_case("who");
        (bearer, words[index..].join(" "), ids, gap)
    };
    if (gap != question && !property_gap)
        || (property_gap && gap)
        || (!question && bearer.is_empty())
    {
        return None;
    }
    let mut roles = std::collections::BTreeMap::new();
    let topic_constraint =
        (property_gap && reference_kind(&bearer).is_none()).then(|| bearer.clone());
    if !gap {
        add_role(&mut roles, ContentSlotIR::Theme, bearer)?;
    }
    Some(DescribedEventIR {
        kind: DescriptionKindIR::State,
        event_id: format!("STATE:{:x}", Sha256::digest(source.as_bytes())),
        lexical_entry_ids: ids,
        predicate_surface: predicate,
        negated,
        roles,
        topic_constraint,
        bearer_kind_constraint,
    })
}

/// A stative -아/어서 clause explaining a stative outcome. Unlike an action
/// sequence, both predicates have source-backed adjective readings. Retain the
/// user's relation as attributed content, never as an independently proved cause.
fn stative_causal_spans(source: &str) -> Option<(&str, &str)> {
    if source.len() > 1024
        || !source.contains("서 ")
        || source.contains(['"', '“', '”', ';', '\n', '?'])
    {
        return None;
    }
    let body = source.trim().trim_end_matches(['.', '!']);
    let last = body.split_whitespace().next_back()?;
    if stative_predicate_ids(last).is_empty() {
        return None;
    }
    let mut split = None;
    for (start, _) in body
        .char_indices()
        .filter(|(i, _)| *i == 0 || body[..*i].ends_with(char::is_whitespace))
    {
        let word = body[start..].split_whitespace().next()?;
        if word.ends_with('서') && !stative_predicate_ids(word).is_empty() {
            let lookup = crate::lexical_knowledge_pack::builtin_pack().lookup(word);
            if lookup.matches.iter().any(|m| {
                m.matched_form == word
                    && m.morphology.grammar_rule == "KO_CONNECTIVE_CAUSE_OR_SEQUENCE"
            }) {
                if split.is_some() {
                    return None;
                }
                split = Some(start + word.len());
            }
        }
    }
    let end = split?;
    let effect = body[end..].trim();
    (!effect.is_empty()).then_some((&body[..end], effect))
}

/// First-person recall of a stated property. The reporting head scopes over
/// the quoted property; the question is not a claim that the property is true.
pub(crate) fn self_reported_cause_target(text: &str) -> Option<&str> {
    let body = text.trim().strip_suffix('?')?;
    let words = body.split_whitespace().collect::<Vec<_>>();
    let [speaker, wh, target, head] = words.as_slice() else {
        return None;
    };
    if !matches!(*speaker, "내가" | "제가") || *wh != "왜" || !target.ends_with("다고") {
        return None;
    }
    let lookup = crate::lexical_knowledge_pack::builtin_pack().lookup(head);
    let reporting = lookup.matches.iter().any(|m| {
        m.matched_form == *head
            && matches!(m.entry.lemma.as_str(), "하다" | "말하다")
            && m.morphology.grammar_rule == "KO_PRINCIPAL_FORM_PAST_ENDING"
    });
    (reporting && !stative_predicate_ids(target).is_empty()).then_some(*target)
}

impl PropositionContentIR {
    pub fn compile(source: &str) -> Self {
        let mut bindings = Vec::new();
        if let Some((value, grammar_evidence)) = desired_content_with_evidence(source) {
            bindings.push(ContentBindingIR {
                event_id: None,
                slot: ContentSlotIR::Intention,
                value,
                predicate: None,
                grammar_evidence: grammar_evidence.into(),
            });
        }
        if !crate::conversation_contract::is_interrogative(source) {
            for condition in crate::modality::ModalSemanticAnalyzer
                .analyze(source)
                .conditionals
            {
                bindings.push(ContentBindingIR {
                    event_id: None,
                    slot: ContentSlotIR::Condition,
                    value: condition.antecedent,
                    predicate: Some(condition.consequent),
                    grammar_evidence: "CONDITIONAL_ANTECEDENT".into(),
                });
            }
        }
        let parsed = CompositionalSemanticAnalyzer.analyze(source);
        let graph = &parsed.semantic_role_graph;
        for edge in &graph.role_edges {
            let slot = match edge.role {
                SemanticRoleKindIR::Agent => ContentSlotIR::Agent,
                SemanticRoleKindIR::Theme | SemanticRoleKindIR::Patient => ContentSlotIR::Theme,
                _ => continue,
            };
            let Some(node) = graph
                .nodes
                .iter()
                .find(|node| node.node_id == edge.argument_node_id)
            else {
                continue;
            };
            if node.kind != crate::semantic_roles::SemanticNodeKindIR::Entity {
                continue;
            }
            bindings.push(ContentBindingIR {
                event_id: None,
                slot,
                value: node.surface.clone(),
                predicate: graph
                    .nodes
                    .iter()
                    .find(|node| node.node_id == edge.event_node_id)
                    .map(|node| node.normalized_label.clone()),
                grammar_evidence: format!("ROLE:{:?}:{}", edge.role, edge.evidence_surface),
            });
        }
        // ASCII folding preserves byte offsets and the original entity casing.
        let lower = source.to_ascii_lowercase();
        if let Some((effect, cause)) = lower.split_once(" because ") {
            if !effect.trim().is_empty() && !cause.trim().is_empty() {
                bindings.push(ContentBindingIR {
                    event_id: None,
                    slot: ContentSlotIR::Cause,
                    value: source[effect.len() + " because ".len()..]
                        .trim()
                        .to_string(),
                    predicate: None,
                    grammar_evidence: "CAUSAL_COMPLEMENT:because".into(),
                });
            }
        } else if let Some((cause, _)) = stative_causal_spans(source) {
            bindings.push(ContentBindingIR {
                event_id: None,
                slot: ContentSlotIR::Cause,
                value: cause.to_string(),
                predicate: None,
                grammar_evidence: "ATTRIBUTED_STATIVE_CAUSAL_CONNECTIVE".into(),
            });
        } else {
            // -서 can mark either sequence or cause. Do not silently select
            // causality from that ending alone. Require an explicit connective.
            for connector in [" 때문에 "] {
                if let Some(position) = lower.find(connector) {
                    if position > 0 && position + connector.len() < lower.len() {
                        bindings.push(ContentBindingIR {
                            event_id: None,
                            slot: ContentSlotIR::Cause,
                            value: source[..position + connector.len()].trim().to_string(),
                            predicate: None,
                            grammar_evidence: format!("CAUSAL_CONNECTIVE:{}", connector.trim()),
                        });
                        break;
                    }
                }
            }
        }
        if let Some((subject, definition)) = lower.split_once(" means ") {
            if !subject.trim().is_empty() && !definition.trim().is_empty() {
                bindings.push(ContentBindingIR {
                    event_id: None,
                    slot: ContentSlotIR::Definition,
                    value: source[subject.len() + " means ".len()..].trim().to_string(),
                    predicate: None,
                    grammar_evidence: "DEFINITION_COMPLEMENT:means".into(),
                });
            }
        }
        let events = described_event(reported_event_surface(source), false)
            .into_iter()
            .collect::<Vec<_>>();
        if let Some(event) = events.first() {
            // The same event owns every role. Do not merge the planner's fallback
            // theme or implicit execution agent into an observed description.
            bindings.retain(|b| !matches!(b.slot, ContentSlotIR::Agent | ContentSlotIR::Theme));
            bindings.extend(event.bindings());
        }
        bindings.truncate(16);
        if is_event_correction_surface(source) {
            bindings
                .retain(|b| matches!(b.slot, ContentSlotIR::Intention | ContentSlotIR::Condition));
        }
        Self {
            source_sha256: format!("{:x}", Sha256::digest(source.as_bytes())),
            bindings,
            events,
            context_sources: Vec::new(),
            interaction_preference: interaction_preference(source),
        }
    }

    pub fn validate_source(&self, source: &str) -> bool {
        Self::compile_contextual(source, &self.context_sources).as_ref() == Some(self)
    }

    pub fn compile_contextual(source: &str, sources: &[EventSourceIR]) -> Option<Self> {
        if sources.is_empty() {
            return Some(Self::compile(source));
        }
        if sources.len() > 8 {
            return None;
        }
        let mut prior = None;
        let mut ids = std::collections::BTreeSet::new();
        for s in sources {
            if s.belief_id.is_empty() || s.source_actor.is_empty() || !ids.insert(&s.belief_id) {
                return None;
            }
            prior = Some(compose_report(&s.source_proposition, prior.as_ref())?);
        }
        let event = compose_report(source, prior.as_ref())?;
        let mut content = Self::compile(source);
        content.bindings.retain(|b| {
            b.event_id.is_none() && !matches!(b.slot, ContentSlotIR::Agent | ContentSlotIR::Theme)
        });
        content.bindings.extend(event.bindings());
        content.events = vec![event];
        content.context_sources = sources.to_vec();
        Some(content)
    }
}

/// Explicit contrastive corrections and referential assertions share the same
/// event-role compiler. No generated answer is stored as an observed proposition.
pub(crate) fn strip_correction_prefix(source: &str) -> (&str, bool) {
    for prefix in [
        "아니, ",
        "아니 ",
        "정정할게. ",
        "정정: ",
        "No, ",
        "no, ",
        "Actually, ",
        "actually, ",
    ] {
        if let Some(rest) = source.strip_prefix(prefix) {
            return (rest, true);
        }
    }
    (source, false)
}

pub(crate) fn compose_report(
    source: &str,
    prior: Option<&DescribedEventIR>,
) -> Option<DescribedEventIR> {
    if let Some((old, new)) = nominal_role_revision(source) {
        let prior = prior?;
        let roles = prior
            .roles
            .iter()
            .filter(|(_, v)| entity_key(v) == entity_key(&old))
            .map(|(s, _)| *s)
            .collect::<Vec<_>>();
        if roles.len() != 1 {
            return None;
        }
        let mut event = prior.clone();
        event.roles.insert(roles[0], new);
        event.event_id = format!("DESCRIBED_EVENT_{:x}", Sha256::digest(source.as_bytes()));
        return Some(event);
    }
    let mut text = source.trim().trim_end_matches(['.', '?', '!']).trim();
    let (rest, correction) = strip_correction_prefix(text);
    text = rest;
    let mut excluded = None;
    if correction {
        if let Some((old, new)) = text.split_once(" 아니라 ") {
            excluded = Some(
                old.strip_suffix('가')
                    .or_else(|| old.strip_suffix('이'))
                    .unwrap_or(old)
                    .trim()
                    .to_string(),
            );
            text = new;
        } else if let Some((new, old)) = text.split_once(", not ") {
            excluded = Some(old.trim().trim_start_matches("the ").to_string());
            text = new;
        }
    }
    let mut event = described_event(text, false)?;
    if correction {
        let prior = prior?;
        if prior.negated != event.negated
            || !prior
                .lexical_entry_ids
                .iter()
                .any(|i| event.lexical_entry_ids.contains(i))
        {
            return None;
        }
        if let Some(old) = excluded {
            let changed = prior
                .roles
                .iter()
                .filter(|(_, v)| entity_key(v) == entity_key(&old))
                .map(|(s, _)| *s)
                .collect::<Vec<_>>();
            if changed.len() != 1 || !event.roles.contains_key(&changed[0]) {
                return None;
            }
            if event.roles.iter().any(|(s, v)| {
                *s != changed[0]
                    && prior
                        .roles
                        .get(s)
                        .is_some_and(|p| entity_key(p) != entity_key(v))
            }) {
                return None;
            }
        } else if event.roles.iter().any(|(s, v)| {
            matches!(s, ContentSlotIR::Agent | ContentSlotIR::Theme)
                && prior
                    .roles
                    .get(s)
                    .is_some_and(|p| entity_key(p) != entity_key(v))
        }) {
            return None;
        }
        for (slot, value) in &prior.roles {
            event.roles.entry(*slot).or_insert_with(|| value.clone());
        }
    }
    if event.has_references() {
        let prior = prior?;
        for (slot, mention) in &mut event.roles {
            let Some(kind) = reference_kind(mention) else {
                continue;
            };
            let candidates = prior
                .roles
                .iter()
                .filter(|(s, v)| match kind {
                    EventReferenceKind::Theme => **s == ContentSlotIR::Theme,
                    EventReferenceKind::Place => {
                        **s == ContentSlotIR::Location
                            || kind.state_bearer_evidence(prior, **s, v).is_some()
                    }
                    EventReferenceKind::Person => {
                        matches!(s, ContentSlotIR::Agent | ContentSlotIR::Recipient)
                            || kind.state_bearer_evidence(prior, **s, v).is_some()
                    }
                    EventReferenceKind::Named(name) => entity_key(v) == entity_key(name),
                })
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
                return None;
            }
            let _ = slot;
            *mention = candidates[0].1.clone();
        }
    }
    event.event_id = format!("DESCRIBED_EVENT_{:x}", Sha256::digest(source.as_bytes()));
    Some(event)
}

pub(crate) fn is_event_correction_surface(source: &str) -> bool {
    nominal_role_revision(source).is_some()
        || ["아니, ", "아니 ", "정정할게. ", "no, ", "actually, "]
            .iter()
            .any(|s| source.trim().to_lowercase().starts_with(s))
}

/// Contrastive nominal replacement binds through an old role value, not through
/// an arbitrary newest event. Both the replaced value and unique event are
/// checked again by contextual compilation/ledger validation.
fn nominal_role_revision(source: &str) -> Option<(String, String)> {
    if source.contains(['?', '"', '“', '”', '`']) {
        return None;
    }
    let text = source.trim().trim_end_matches(['.', '!']);
    let lower = text.to_ascii_lowercase();
    if let Some(complement) = lower
        .strip_prefix("it was ")
        .or_else(|| lower.strip_prefix("it is "))
    {
        let (new, old) = complement.split_once(", not ")?;
        let nominal = |value: &str| {
            let value = value
                .strip_prefix("the ")
                .or_else(|| value.strip_prefix("a "))
                .or_else(|| value.strip_prefix("an "))
                .unwrap_or(value)
                .trim();
            (!value.is_empty()
                && value.split_whitespace().count() <= 3
                && !value.contains(['.', ':', ';', ',', '\n'])
                && reference_kind(value).is_none())
            .then_some(value.to_string())
        };
        return Some((nominal(old)?, nominal(new)?));
    }
    let clause = text.rsplit(". ").next()?;
    let (old, new) = clause.split_once(" 아니고 ")?;
    let old = old.strip_suffix('이').or_else(|| old.strip_suffix('가'))?;
    let new = ["입니다", "이에요", "예요", "이야", "야"]
        .iter()
        .find_map(|s| new.strip_suffix(s))?;
    if old.is_empty()
        || new.is_empty()
        || old.split_whitespace().count() > 2
        || new.split_whitespace().count() > 2
        || reference_kind(new).is_some()
    {
        return None;
    }
    Some((old.into(), new.into()))
}

// Syntactic normalization retains the lexical predicate and explicit roles.
// It produces another question, never an answer or an executable command.
fn normalize_role_question(source: &str) -> String {
    let source = crate::conversation::topic_question_parts(source).map_or(source, |(_, body)| body);
    let lower = source
        .trim()
        .trim_end_matches(['?', '.', '!'])
        .to_lowercase();
    let mut text = lower.as_str();
    for prefix in ["그럼 ", "그러면 ", "그렇다면 ", "then ", "and "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest;
        }
    }
    text = text.strip_suffix(", again").unwrap_or(text);
    if let Some(rest) = text.strip_prefix("which person ") {
        return format!("who {}", rest.replace("didn't ", "did not "));
    }
    if !text.chars().any(|c| ('가'..='힣').contains(&c)) {
        return text.replace("didn't ", "did not ");
    }
    // A role noun is a query gap only in a fully consumed nominal question.
    // Polite request morphology wraps the same gap; it is not an action goal.
    for ending in [
        " 같이 알려줘",
        " 알려줄래",
        " 알려주세요",
        " 알려줘",
        " 어디라고",
        " 누구였지",
        " 누구인가요",
        " 누구야",
    ] {
        if let Some(rest) = text.strip_suffix(ending) {
            text = rest;
            break;
        }
    }
    // Additive request adjuncts compose with every politeness ending; they are
    // not part of the queried role nouns or a separate sentence template.
    for adjunct in [" 같이", " 함께"] {
        text = text.strip_suffix(adjunct).unwrap_or(text);
    }
    if text.contains(' ') {
        // Coordinated head nouns may have spaces: find the first role head
        // after the indexed predicate, then consume all remaining head tokens.
        let words = text.split_whitespace().collect::<Vec<_>>();
        for split in 1..words.len() {
            let slots = words[split..]
                .iter()
                .map(|w| korean_role_head(w))
                .collect::<Option<Vec<_>>>();
            if let Some(slots) = slots {
                let clause = words[..split].join(" ");
                let mut gaps = slots.iter().map(|s| role_gap(*s)).collect::<Vec<_>>();
                gaps.dedup();
                // A bare recipient noun denotes the focus role, without
                // asserting that the source event's predicate was RECEIVE.
                if matches!(clause.as_str(), "받은" | "받는") && slots == [ContentSlotIR::Agent]
                {
                    return "누구에게".into();
                }
                return format!("{} {clause}", gaps.join(" "));
            }
        }
    }
    text.to_string()
}

fn korean_role_head(word: &str) -> Option<ContentSlotIR> {
    let stem = korean_nominal_head(word);
    Some(match stem {
        "사람" | "쪽" => ContentSlotIR::Agent,
        "장소" | "곳" => ContentSlotIR::Location,
        "대상" | "물건" => ContentSlotIR::Theme,
        "시간" => ContentSlotIR::Time,
        _ => return None,
    })
}

/// Nominal information-role heads used by scoped negative response acts.
/// This does not turn an arbitrary nominal assertion into a question.
pub(crate) fn nominal_response_slot(text: &str) -> Option<ContentSlotIR> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    let words = words.as_slice();
    let words = if matches!(words.first(), Some(&"me" | &"us")) {
        &words[1..]
    } else {
        words
    };
    let words = if words.first() == Some(&"the") {
        &words[1..]
    } else {
        words
    };
    if words.len() != 1 {
        return None;
    }
    korean_role_head(words[0])
        .or_else(|| relational_head(words[0]))
        .or_else(|| {
            Some(match words[0].to_lowercase().as_str() {
                "place" | "location" => ContentSlotIR::Location,
                "time" => ContentSlotIR::Time,
                "duration" | "기간" => ContentSlotIR::Duration,
                _ => return None,
            })
        })
}

fn korean_nominal_head(word: &str) -> &str {
    [
        "이랑", "랑", "와", "과", "은", "는", "이", "가", "을", "를", "도", "만",
    ]
    .iter()
    .find_map(|s| word.strip_suffix(s))
    .unwrap_or(word)
}

fn role_gap(slot: ContentSlotIR) -> &'static str {
    match slot {
        ContentSlotIR::Agent => "누가",
        ContentSlotIR::Location => "어디서",
        ContentSlotIR::Time => "언제",
        _ => "무엇을",
    }
}

/// Shared syntactic ownership predicate. It creates no answer and consults no
/// dialogue entities. The same parse is later checked against attributed memory.
pub(crate) fn is_event_question(source: &str) -> bool {
    if crate::grammatical_scope::embedded_information_statement(source).is_some() {
        return false;
    }
    let Some(event) = described_event(source, true) else {
        return false;
    };
    requested_content_slot(source).is_some()
        && (crate::conversation_contract::is_interrogative(source)
            || event.kind == DescriptionKindIR::State && event.lexical_entry_ids.is_empty()
            || normalize_role_question(source) != source.trim().to_lowercase())
}

// Passive syntax licenses a participle, not an arbitrary verb or an -ed suffix.
// Both voices retain the original dictionary entry identities.
fn participle_entries(word: &str) -> Vec<String> {
    let lookup = crate::lexical_knowledge_pack::builtin_pack().lookup(word);
    if lookup.truncated {
        return vec![];
    }
    let mut ids = lookup
        .matches
        .into_iter()
        .filter(|m| {
            m.matched_form == word
                && m.entry.pos == "동사"
                && (matches!(
                    m.morphology.grammar_rule.as_str(),
                    "EN_REGULAR_PAST" | "EN_PAST_PARTICIPLE"
                ) || m.entry.senses.iter().any(|s| {
                    s.english.split(';').any(|a| {
                        crate::lexical_knowledge_pack::english_participle(a.trim()) == Some(word)
                    })
                }))
        })
        .map(|m| m.entry.source_entry_id)
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn passive_preposition(word: &str) -> Option<ContentSlotIR> {
    Some(match word {
        "by" => ContentSlotIR::Agent,
        "to" => ContentSlotIR::Recipient,
        "from" => ContentSlotIR::Source,
        "in" | "at" => ContentSlotIR::Location,
        "for" => ContentSlotIR::Duration,
        _ => return None,
    })
}

fn passive_nominal(words: &[&str]) -> String {
    let words = if words
        .first()
        .is_some_and(|w| matches!(*w, "the" | "a" | "an"))
    {
        &words[1..]
    } else {
        words
    };
    words.join(" ")
}

// One role assembler is shared by passive observations and passive questions.
// A stranded preposition is a gap only in the role actually requested.
fn passive_roles(
    theme: &[&str],
    tail: &[&str],
    requested: Option<ContentSlotIR>,
) -> Option<std::collections::BTreeMap<ContentSlotIR, String>> {
    if theme
        .iter()
        .any(|w| *w == "not" || passive_preposition(w).is_some())
    {
        return None;
    }
    let mut roles = std::collections::BTreeMap::new();
    add_role(&mut roles, ContentSlotIR::Theme, passive_nominal(theme))?;
    let mut cursor = 0;
    while cursor < tail.len() {
        let role = passive_preposition(tail[cursor])?;
        cursor += 1;
        let start = cursor;
        while cursor < tail.len() && passive_preposition(tail[cursor]).is_none() {
            cursor += 1;
        }
        if cursor == start {
            if cursor == tail.len() && requested == Some(role) {
                break;
            }
            return None;
        }
        add_role(&mut roles, role, passive_nominal(&tail[start..cursor]))?;
    }
    Some(roles)
}

fn passive_role_question(text: &str, source: &str) -> Option<DescribedEventIR> {
    let lower = text.to_lowercase();
    let words = lower.split_whitespace().collect::<Vec<_>>();
    let head = match words.first().copied()? {
        "who" | "where" | "when" => 1,
        "how" if words.get(1) == Some(&"long") => 2,
        _ => return None,
    };
    if !words
        .get(head)
        .is_some_and(|w| matches!(*w, "was" | "were" | "is" | "are"))
    {
        return None;
    }
    let requested = requested_content_slot(source)?;
    if words[0] == "who"
        && words
            .last()
            .and_then(|w| passive_preposition(w))
            .is_none_or(|r| r != requested)
    {
        return None;
    }
    let mut candidates = Vec::new();
    for i in head + 2..words.len() {
        let ids = participle_entries(words[i]);
        if ids.is_empty() {
            continue;
        }
        let negated = words[i - 1] == "not";
        let theme_end = if negated { i - 1 } else { i };
        let Some(roles) = passive_roles(
            &words[head + 1..theme_end],
            &words[i + 1..],
            Some(requested),
        ) else {
            continue;
        };
        // A requested relation is unfilled, not a contradictory supplied value.
        if roles.contains_key(&requested) {
            continue;
        }
        candidates.push(DescribedEventIR {
            kind: DescriptionKindIR::Event,
            event_id: format!("DESCRIBED_EVENT_{:x}", Sha256::digest(source.as_bytes())),
            lexical_entry_ids: ids,
            predicate_surface: words[i].into(),
            negated,
            roles,
            topic_constraint: None,
            bearer_kind_constraint: None,
        });
    }
    (candidates.len() == 1).then(|| candidates.remove(0))
}

/// Passive voice changes grammatical order, not event roles. Observations still
/// require an explicit by-agent; questions may ask about an unfilled role.
fn passive_event_report(text: &str, source: &str) -> Option<DescribedEventIR> {
    let lower = text.to_lowercase();
    let words = lower.split_whitespace().collect::<Vec<_>>();
    let auxiliary = words.iter().position(|w| matches!(*w, "was" | "were"))?;
    let mut i = auxiliary + 1;
    let negated = words.get(i) == Some(&"not");
    if negated {
        i += 1;
    }
    let predicate = *words.get(i)?;
    let ids = participle_entries(predicate);
    if ids.is_empty() {
        return None;
    }
    let roles = passive_roles(&words[..auxiliary], &words[i + 1..], None)?;
    // Agent presence is a semantic requirement, not a requirement that its
    // prepositional phrase precede every location or duration adjunct.
    if !roles.contains_key(&ContentSlotIR::Agent) {
        return None;
    }
    Some(DescribedEventIR {
        kind: DescriptionKindIR::Event,
        event_id: format!("DESCRIBED_EVENT_{:x}", Sha256::digest(source.as_bytes())),
        lexical_entry_ids: ids,
        predicate_surface: predicate.into(),
        negated,
        roles,
        topic_constraint: None,
        bearer_kind_constraint: None,
    })
}

pub fn requested_content_slot(text: &str) -> Option<ContentSlotIR> {
    requested_content_slots(text).into_iter().next()
}

/// A relational question with no new content referent asks about the current
/// proposition. Politeness and the matrix predicate do not become its target.
/// Explicit complements ("the cause of X", "why X happened") do not pass.
pub(crate) fn contextual_content_slot(text: &str) -> Option<ContentSlotIR> {
    content_request(text)
        .filter(|request| request.target_surface.is_none())
        .map(|request| request.slot)
}

/// One source-bound content request shared by retrieval and gap realization.
/// Target identity and requested expression style are independent axes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentRequestIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_lead_in: Option<String>,
    pub source_text: String,
    pub slot: ContentSlotIR,
    pub argument_surface: String,
    /// None denotes a contextual relational argument, not an arbitrary wildcard.
    pub target_surface: Option<String>,
    pub response_manner: Option<ResponseMannerIR>,
}

impl ContentRequestIR {
    pub fn validate(&self) -> bool {
        content_request(&self.source_text).as_ref() == Some(self)
    }
}

/// The addressed request and its inner question are separate source-bound
/// objects. The query engine sees the inner question, never a communication
/// predicate masquerading as an event to retrieve or execute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionRequestIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clause_context: Option<QuestionClauseContextIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_lead_in: Option<String>,
    pub source_text: String,
    pub question_text: String,
    pub response_manner: Option<ResponseMannerIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionClauseContextIR {
    pub turn_text: String,
    pub frame_id: String,
}

impl QuestionRequestIR {
    pub fn validate(&self) -> bool {
        if let Some(context) = &self.clause_context {
            let analysis = CompositionalSemanticAnalyzer.analyze(&context.turn_text);
            question_request_for_frame(&context.turn_text, &analysis, &context.frame_id).as_ref()
                == Some(self)
        } else {
            question_request(&self.source_text).as_ref() == Some(self)
        }
    }
}

pub(crate) fn question_request(text: &str) -> Option<QuestionRequestIR> {
    question_from_argument(text, response_argument(text)?)
}

/// Compile a request from the authoritative turn analysis. Do not reparse a
/// clause fragment: its complement ownership and directive force are already
/// known, including when the governor participates in coordination.
pub(crate) fn question_request_for_frame(
    text: &str,
    analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
    frame_id: &str,
) -> Option<QuestionRequestIR> {
    let node = analysis.clause_graph.node_for_frame(frame_id)?;
    if !node.function.permits_independent_directive() {
        return None;
    }
    let mut request = question_from_argument(
        &node.source_text,
        response_argument_from_analysis(text, analysis, Some(frame_id))?,
    )?;
    request.clause_context = Some(QuestionClauseContextIR {
        turn_text: text.to_string(),
        frame_id: frame_id.to_string(),
    });
    Some(request)
}

fn question_from_argument(text: &str, argument: ResponseArgument) -> Option<QuestionRequestIR> {
    let ResponseArgument {
        argument,
        response_manner,
        wrapped,
        request_lead_in,
    } = argument;
    if !wrapped {
        return None;
    }
    let mut question = argument;
    if question.chars().any(|c| ('가'..='힣').contains(&c)) {
        // Complement morphology is removed only at the final predicate.
        // The existing indexed predicate/role grammar must consume the result.
        let (prefix, predicate) = question.rsplit_once(char::is_whitespace)?;
        let stem = predicate.strip_suffix("는지")?;
        question = format!("{prefix} {stem}다");
    } else if let Some(rest) = question.strip_prefix("what ") {
        // Embedded object questions use subject-verb order. Supply only the
        // grammatical auxiliary expected by the existing direct-query parser.
        if !rest.starts_with("did ") {
            question = format!("what did {rest}");
        }
    }
    question.push('?');
    let slots = requested_content_slots(&question);
    if slots.is_empty()
        || slots.iter().any(|slot| {
            !matches!(
                slot,
                ContentSlotIR::Agent
                    | ContentSlotIR::Theme
                    | ContentSlotIR::Recipient
                    | ContentSlotIR::Source
                    | ContentSlotIR::Location
                    | ContentSlotIR::Time
                    | ContentSlotIR::Duration
            )
        })
        || !is_event_question(&question)
    {
        return None;
    }
    Some(QuestionRequestIR {
        clause_context: None,
        request_lead_in,
        source_text: text.to_string(),
        question_text: question,
        response_manner,
    })
}

pub(crate) fn take_response_manner<'a>(
    mut text: &'a str,
    manner: &mut Option<ResponseMannerIR>,
) -> Option<&'a str> {
    // A degree/softener modifies the manner head, never the content argument.
    // At most one phrase at each boundary; all phrase tokens must be consumed.
    for _ in 0..2 {
        let boundaries = text
            .char_indices()
            .filter(|(_, c)| c.is_whitespace())
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect::<Vec<_>>();
        let found = boundaries
            .iter()
            .rev()
            .find_map(|&i| manner_phrase(&text[..i]).map(|value| (text[i..].trim(), value)))
            .or_else(|| {
                boundaries.iter().find_map(|&i| {
                    manner_phrase(text[i..].trim()).map(|value| (text[..i].trim(), value))
                })
            });
        if let Some((rest, value)) = found {
            if manner.is_some_and(|existing| existing != value) {
                return None;
            }
            *manner = Some(value);
            text = rest;
        } else {
            break;
        }
    }
    Some(text)
}

fn manner_phrase(text: &str) -> Option<ResponseMannerIR> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() || words.len() > 5 {
        return None;
    }
    let (head, modifiers) = words.split_last()?;
    if *head == "detail" {
        let degrees = modifiers.strip_prefix(&["in"])?;
        return degrees
            .iter()
            .all(|w| matches!(*w, "more" | "much" | "greater"))
            .then_some(ResponseMannerIR::Detailed);
    }
    if !modifiers
        .iter()
        .all(|w| matches!(*w, "좀" | "조금" | "더" | "아주" | "more"))
    {
        return None;
    }
    Some(match *head {
        "자세히" => ResponseMannerIR::Detailed,
        "briefly" | "concisely" | "짧게" | "간단히" => ResponseMannerIR::Concise,
        _ => match crate::language_knowledge::korean_derived_response_manner(head) {
            Some(crate::language_knowledge::LanguageDialogueDirectiveValueIR::Detailed) => {
                ResponseMannerIR::Detailed
            }
            Some(crate::language_knowledge::LanguageDialogueDirectiveValueIR::Concise) => {
                ResponseMannerIR::Concise
            }
            _ => return None,
        },
    })
}

/// An interrogative complement describes an information gap, not an action to
/// perform. Referential event predicates bind to discourse, not to a solution.
fn embedded_relation(argument: &str) -> Option<(ContentSlotIR, Option<String>)> {
    let (wh, rest) = argument.split_once(char::is_whitespace)?;
    let slot = match wh {
        "why" | "왜" => ContentSlotIR::Cause,
        "how" | "어떻게" => ContentSlotIR::Manner,
        _ => return None,
    };
    let rest = rest.trim();
    let words = rest.split_whitespace().collect::<Vec<_>>();
    let contextual = match words.as_slice() {
        [reference, event] => {
            matches!(*reference, "it" | "that" | "this")
                && matches!(*event, "happened" | "occurred")
        }
        [event] => ["는지", "어요", "어"]
            .iter()
            .find_map(|ending| event.strip_suffix(ending))
            .is_some_and(|stem| matches!(stem, "그랬" | "그랬었")),
        _ => false,
    };
    if contextual {
        return Some((slot, None));
    }
    // Explicit finite English clauses remain source targets. Korean inflected
    // explicit clauses require a morphological event join, not suffix guessing.
    if wh.is_ascii()
        && words.len() >= 2
        && !words
            .iter()
            .any(|w| matches!(*w, "it" | "that" | "this" | "and" | "then" | "not"))
    {
        return Some((slot, Some(rest.to_string())));
    }
    None
}

fn relational_head(head: &str) -> Option<ContentSlotIR> {
    Some(match korean_nominal_head(head) {
        "why" | "cause" | "reason" | "왜" | "이유" | "원인" => ContentSlotIR::Cause,
        "how" | "method" | "manner" | "어떻게" | "방법" | "방식" => ContentSlotIR::Manner,
        "definition" | "meaning" | "정의" | "의미" => ContentSlotIR::Definition,
        _ => return None,
    })
}

/// A relation-valued question separates its open relation from the clause it
/// qualifies. This is a borrowed syntactic view, never a fact or an answer.
pub(crate) struct NominalRelationQuestion<'a> {
    pub slot: ContentSlotIR,
    pub complement: &'a str,
    pub korean_attributive: bool,
}

pub(crate) fn nominal_relation_question(text: &str) -> Option<NominalRelationQuestion<'_>> {
    if text.len() > 8192 || text.contains(['"', '“', '”', '‘', '’', '`']) {
        return None;
    }
    let text = text.trim().trim_end_matches(['?', '.', '!']).trim();
    if let Some(body) = text.strip_prefix("what is ") {
        let body = body.strip_prefix("the ").unwrap_or(body);
        let (head, rest) = body.split_once(' ')?;
        let slot = relational_head(head)?;
        let (link, complement) = rest.split_once(' ')?;
        if !matches!(link, "that" | "why") || slot != ContentSlotIR::Cause {
            return None;
        }
        return (!complement.trim().is_empty()).then_some(NominalRelationQuestion {
            slot,
            complement: complement.trim(),
            korean_attributive: false,
        });
    }
    let (body, copula) = text.rsplit_once(char::is_whitespace)?;
    // Interrogative pronoun + copular ending, not a whole-sentence pattern.
    let ending = copula
        .strip_prefix("무엇")
        .or_else(|| copula.strip_prefix("뭐"))?;
    if !matches!(
        ending,
        "야" | "예요" | "이야" | "이에요" | "인가" | "인가요" | "입니까"
    ) {
        return None;
    }
    let (complement, head) = body.trim().rsplit_once(char::is_whitespace)?;
    let head = head.strip_suffix(['이', '가', '은', '는']).unwrap_or(head);
    Some(NominalRelationQuestion {
        slot: relational_head(head)?,
        complement: complement.trim(),
        korean_attributive: true,
    })
}

struct ResponseArgument {
    argument: String,
    response_manner: Option<ResponseMannerIR>,
    wrapped: bool,
    request_lead_in: Option<String>,
}

fn response_argument(text: &str) -> Option<ResponseArgument> {
    let analysis = CompositionalSemanticAnalyzer.analyze(&text.to_lowercase());
    response_argument_from_analysis(text, &analysis, None)
}

fn response_argument_from_analysis(
    text: &str,
    analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
    selected_frame: Option<&str>,
) -> Option<ResponseArgument> {
    use crate::compositional_semantics::{FrameMoodIR, FramePolarityIR};
    if text.contains(['"', '“', '”', '`']) {
        return None;
    }
    let source = text.to_lowercase();
    let mut response_manner = None;
    let mut request_lead_in = None;
    let mut has_non_addressee_recipient = false;
    let argument = if analysis.frames.is_empty() {
        if !crate::conversation_contract::is_interrogative(text) {
            return None;
        }
        text.trim().trim_end_matches(['?', '.', '!']).to_lowercase()
    } else {
        let roots = analysis
            .frames
            .iter()
            .filter(|f| {
                selected_frame.is_none_or(|id| id == f.frame_id)
                    && analysis
                        .clause_graph
                        .node_for_frame(&f.frame_id)
                        .is_some_and(|n| n.function.permits_independent_directive())
            })
            .collect::<Vec<_>>();
        if roots.len() != 1 {
            return None;
        }
        let frame = roots[0];
        if analysis.frames.iter().any(|f| {
            if selected_frame.is_some()
                && analysis
                    .clause_graph
                    .owner_for_frame(&f.frame_id)
                    .is_none_or(|owner| owner.anchor_frame_id != frame.frame_id)
            {
                return false;
            }
            f.frame_id != frame.frame_id
                && !analysis
                    .clause_graph
                    .node_for_frame(&f.frame_id)
                    .is_some_and(|n| {
                        n.function == crate::clause_graph::ClauseFunctionIR::ContentComplement
                    })
        }) {
            return None;
        }
        if frame.embedded_under_quote
            || !analysis
                .clause_graph
                .node_for_frame(&frame.frame_id)
                .is_some_and(|node| node.function.permits_independent_directive())
            || frame.polarity != FramePolarityIR::Positive
            || !matches!(
                frame.intent_hint,
                dockable_semantic_core::PlanIntentIR::Explain
                    | dockable_semantic_core::PlanIntentIR::Investigate
                    | dockable_semantic_core::PlanIntentIR::Communicate
            )
            || !(frame.mood == FrameMoodIR::Imperative
                || analysis.modal_scope_graph.is_polite_request())
        {
            return None;
        }
        has_non_addressee_recipient = analysis
            .semantic_role_graph
            .arguments_for_frame(&frame.frame_id)
            .iter()
            .any(|(role, node)| {
                matches!(
                    role,
                    SemanticRoleKindIR::Recipient | SemanticRoleKindIR::Destination
                ) && !matches!(
                    node.normalized_label.as_str(),
                    "me" | "us" | "나" | "저" | "내" | "제" | "우리"
                )
            });
        // A frame theme may be lossy (for example, it may omit a Korean
        // genitive modifier). Consume the actual source argument instead.
        let node = analysis.clause_graph.node_for_frame(&frame.frame_id)?;
        let (start, end) = if selected_frame.is_some() {
            (node.source_start_byte, node.source_end_byte)
        } else {
            (0, source.len())
        };
        let prefix = source.get(start..frame.source_start_byte)?.trim();
        // Only a validated positive, addressed matrix request licenses a
        // discourse prefix here. Never strip it inside the content clause.
        let request_prefix =
            crate::compositional_semantics::strip_conversational_directive_lead_in(prefix);
        if request_prefix.len() != prefix.len() {
            request_lead_in = Some(
                prefix[..prefix.len() - request_prefix.len()]
                    .trim()
                    .to_string(),
            );
        }
        let prefix = request_prefix;
        let tail = source.get(frame.source_start_byte..end)?;
        if !tail.starts_with(&frame.predicate_surface) {
            return None;
        }
        if frame
            .predicate_surface
            .chars()
            .any(|c| ('가'..='힣').contains(&c))
        {
            let verb_end = tail.find(char::is_whitespace).unwrap_or(tail.len());
            if !tail.get(verb_end..)?.trim().is_empty()
                && !crate::compositional_semantics::korean_request_tail(
                    tail.get(frame.predicate_surface.len()..)?,
                )
            {
                return None;
            }
            prefix.to_string()
        } else {
            let prefix = take_response_manner(prefix, &mut response_manner)?;
            if !prefix
                .split_whitespace()
                .all(|w| matches!(w, "please" | "could" | "would" | "can" | "you"))
            {
                return None;
            }
            let rest = tail.get(frame.predicate_surface.len()..)?;
            if !rest.starts_with(char::is_whitespace) {
                return None;
            }
            rest.trim().trim_end_matches(['?', '.', '!']).to_string()
        }
    };
    let argument = ["me ", "us ", "나에게 ", "저에게 ", "나한테 ", "저한테 "]
        .iter()
        .find_map(|prefix| argument.strip_prefix(prefix))
        .unwrap_or(&argument);
    let argument = take_response_manner(argument, &mut response_manner)?;
    let result = ResponseArgument {
        argument: argument.to_string(),
        response_manner,
        wrapped: !analysis.frames.is_empty(),
        request_lead_in,
    };
    // A requested event question owns its participants, including recipients.
    // The general frame analyzer may have no action frame for its lexical
    // predicate and assign a complement's "to X" to the communication frame.
    // Accept that role only when the complete, addressed content is consumed
    // by the event-question grammar. "Tell Alice ..." is not stripped into an
    // addressed request and cannot pass this proof.
    if has_non_addressee_recipient
        && (!result
            .argument
            .split_whitespace()
            .next()
            .is_some_and(|word| {
                matches!(
                    word,
                    "who"
                        | "what"
                        | "where"
                        | "when"
                        | "누가"
                        | "무엇을"
                        | "어디서"
                        | "어디에서"
                        | "언제"
                )
            })
            || question_from_argument(
                text,
                ResponseArgument {
                    argument: result.argument.clone(),
                    response_manner: result.response_manner,
                    wrapped: result.wrapped,
                    request_lead_in: result.request_lead_in.clone(),
                },
            )
            .is_none())
    {
        return None;
    }
    Some(result)
}

pub(crate) fn content_request(text: &str) -> Option<ContentRequestIR> {
    let ResponseArgument {
        argument,
        response_manner,
        wrapped,
        request_lead_in,
    } = response_argument(text)?;
    let argument = argument.as_str();
    if let Some((slot, target_surface)) =
        embedded_relation(argument).filter(|(_, target)| wrapped || target.is_none())
    {
        return Some(ContentRequestIR {
            request_lead_in,
            source_text: text.to_string(),
            slot,
            argument_surface: argument.to_string(),
            target_surface,
            response_manner,
        });
    }
    let words = argument.split_whitespace().collect::<Vec<_>>();
    let (head, modifiers) = words.split_last()?;
    let deictic = |modifiers: &[&str]| {
        modifiers.len() <= 1
            && modifiers.iter().all(|m| {
                matches!(
                    *m,
                    "the" | "its" | "that" | "this" | "그" | "그것의" | "그거의" | "이"
                )
            })
    };
    let (slot, target_surface, argument_surface) =
        if let Some(slot) = relational_head(head.trim_end_matches(['?', '.', '!'])) {
            let target = if deictic(modifiers) {
                None
            } else {
                // Korean genitive modifies the relational head. Do not silently
                // discard other modifiers or reinterpret arbitrary noun order.
                if !modifiers.last().is_some_and(|w| w.ends_with('의')) {
                    return None;
                }
                Some(
                    modifiers
                        .join(" ")
                        .trim_end_matches('의')
                        .trim()
                        .to_string(),
                )
            };
            (
                slot,
                target,
                modifiers
                    .iter()
                    .copied()
                    .chain(std::iter::once(korean_nominal_head(head)))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        } else {
            let (relation, target) = argument.split_once(" of ")?;
            let relation_words = relation.split_whitespace().collect::<Vec<_>>();
            let (head, modifiers) = relation_words.split_last()?;
            if !deictic(modifiers) || target.trim().is_empty() {
                return None;
            }
            (
                relational_head(head)?,
                Some(target.trim().to_string()),
                argument.to_string(),
            )
        };
    Some(ContentRequestIR {
        request_lead_in,
        source_text: text.to_string(),
        slot,
        argument_surface,
        target_surface,
        response_manner,
    })
}

pub fn requested_content_slots(text: &str) -> Vec<ContentSlotIR> {
    if let Some(state) = described_state(text, true) {
        return vec![if state.lexical_entry_ids.is_empty() {
            ContentSlotIR::Property
        } else {
            ContentSlotIR::Theme
        }];
    }
    let primary = requested_single_slot(text);
    let mut slots = primary.into_iter().collect::<Vec<_>>();
    let lower = normalize_role_question(text);
    let words = lower
        .split(|c: char| !c.is_alphanumeric())
        .collect::<Vec<_>>();
    for (slot, markers) in [
        (ContentSlotIR::Agent, &["누가", "who"][..]),
        (
            ContentSlotIR::Location,
            &["어디서", "어디에서", "where"][..],
        ),
        (ContentSlotIR::Time, &["언제", "when"][..]),
        (ContentSlotIR::Theme, &["뭘", "무엇을"][..]),
    ] {
        if markers.iter().any(|m| words.contains(m))
            && !slots.contains(&slot)
            && !(slot == ContentSlotIR::Agent && primary == Some(ContentSlotIR::Recipient))
        {
            slots.push(slot);
        }
    }
    // Postverbal bare "what" with a subject WH is an object gap; this does
    // not consume "what time", embedded clauses or arbitrary content words.
    if primary == Some(ContentSlotIR::Agent)
        && words.last() == Some(&"what")
        && words.iter().filter(|word| **word == "what").count() == 1
        && !slots.contains(&ContentSlotIR::Theme)
    {
        slots.push(ContentSlotIR::Theme);
    }
    slots
}

fn requested_single_slot(text: &str) -> Option<ContentSlotIR> {
    let lower = normalize_role_question(text);
    if attributed_interaction_preference_query(text) {
        return Some(ContentSlotIR::Intention);
    }
    let self_reference = lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| matches!(w, "내가" | "제가" | "나는" | "난" | "저는" | "i" | "my"));
    if self_reference
        && crate::conversation_contract::is_interrogative(text)
        && ["바라는", "원하는", "원하", "want", "asking"]
            .iter()
            .any(|s| lower.contains(s))
    {
        return Some(ContentSlotIR::Intention);
    }
    if (lower.contains("원하")
        || lower.contains("원하는")
        || lower.contains("want")
        || lower.contains("바라는"))
        && ["뭘", "무엇", "뭐", "what"]
            .iter()
            .any(|s| lower.contains(s))
    {
        return Some(ContentSlotIR::Intention);
    }
    if lower.contains("조건") || lower.contains("condition") {
        return Some(ContentSlotIR::Condition);
    }
    let words = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if lower.contains("받은 사람")
        || lower.contains("받는 사람")
        || lower.starts_with("who ") && lower.trim_end_matches('?').ends_with(" to")
        || lower.starts_with("to whom")
        || lower.starts_with("who did") && lower.ends_with(" to?")
        || words.iter().any(|w| matches!(*w, "누구에게" | "누구한테"))
    {
        return Some(ContentSlotIR::Recipient);
    }
    if lower.starts_with("from whom")
        || words
            .iter()
            .any(|w| matches!(*w, "누구에게서" | "누구한테서"))
    {
        return Some(ContentSlotIR::Source);
    }
    if lower.starts_with("how long") || words.iter().any(|w| matches!(*w, "얼마나" | "몇시간"))
    {
        return Some(ContentSlotIR::Duration);
    }
    if words.iter().any(|w| matches!(*w, "when" | "언제")) {
        return Some(ContentSlotIR::Time);
    }
    if words
        .iter()
        .any(|w| matches!(*w, "where" | "어디에서" | "어디서"))
    {
        return Some(ContentSlotIR::Location);
    }
    if words.iter().any(|word| {
        matches!(
            korean_nominal_head(word),
            "why" | "reason" | "cause" | "왜" | "이유" | "원인"
        )
    }) {
        return Some(ContentSlotIR::Cause);
    }
    if words
        .first()
        .is_some_and(|word| matches!(*word, "who" | "누구"))
        || words.contains(&"누가")
        || lower.contains("사람은 누구")
    {
        return Some(ContentSlotIR::Agent);
    }
    if words.iter().any(|word| matches!(*word, "뭘" | "무엇을"))
        || lower.starts_with("what did ")
        || lower.trim_end_matches('?') == "what"
    {
        return Some(ContentSlotIR::Theme);
    }
    if lower.starts_with("what is ")
        || lower.contains("what a ")
        || lower.contains("뭔지")
        || lower.contains("정의")
    {
        return Some(ContentSlotIR::Definition);
    }
    if words.iter().any(|word| matches!(*word, "how" | "어떻게")) {
        return Some(ContentSlotIR::Manner);
    }
    if words.iter().any(|word| {
        matches!(
            *word,
            "요약"
                | "요약을"
                | "summarize"
                | "summarise"
                | "summary"
                | "요약해"
                | "요약해줘"
                | "요약해줄래"
                | "요약해줄래요"
                | "요약해주세요"
        )
    }) {
        return Some(ContentSlotIR::Summary);
    }
    None
}

/// A Korean matrix question such as "뭘 줄여 달라는 거야?" asks for the
/// speaker's previously stated response preference. The open-class predicate
/// is licensed by the lexical pack; only the reportative request construction
/// and its deictic/source positions are closed grammar.
pub(crate) fn attributed_interaction_preference_query(text: &str) -> bool {
    attributed_interaction_preference_predicate(text).is_some()
}

fn attributed_interaction_preference_predicate(text: &str) -> Option<String> {
    if text.contains(['"', '“', '”', '‘', '’', '`'])
        || !crate::conversation_contract::is_interrogative(text)
    {
        return None;
    }
    let normalized = normalize_role_question(text);
    let words = normalized
        .trim_end_matches(['?', '!', '.'])
        .split_whitespace()
        .collect::<Vec<_>>();
    let wh = words
        .iter()
        .position(|word| matches!(*word, "뭘" | "무엇을" | "뭐를"))?;
    if !words[..wh].iter().all(|word| {
        matches!(
            *word,
            "그러니까"
                | "그래서"
                | "그럼"
                | "지금"
                | "지금은"
                | "이번에는"
                | "내가"
                | "제가"
                | "나는"
                | "저는"
                | "도대체"
                | "정확히"
        )
    }) {
        return None;
    }
    let report = words.iter().position(|word| *word == "달라는")?;
    if report != wh + 2
        || !matches!(
            words.get(report + 1).copied(),
            Some("거야" | "거예요" | "겁니까" | "뜻이야" | "뜻이에요" | "뜻입니까")
        )
        || report + 2 != words.len()
    {
        return None;
    }
    let predicate = words[wh + 1];
    crate::lexical_knowledge_pack::builtin_pack()
        .lookup(predicate)
        .matches
        .iter()
        .find(|matched| {
            matched.matched_form == predicate
                && matches!(matched.entry.pos.as_str(), "동사" | "보조 동사")
        })
        .map(|matched| matched.entry.lemma.clone())
}

pub(crate) fn interaction_preference_answer_focus(
    question: &str,
    preference: &InteractionPreferenceIR,
) -> Option<InteractionPreferenceAnswerFocusIR> {
    if requested_content_slot(question) != Some(ContentSlotIR::Intention) {
        return None;
    }
    let mut mentioned = interaction_modes(question);
    mentioned.sort();
    mentioned.dedup();
    if mentioned.len() >= 2 && mentioned.contains(&preference.desired) {
        let rejected = preference
            .excluded
            .iter()
            .copied()
            .find(|mode| mentioned.contains(mode) && *mode != preference.desired)
            .or_else(|| {
                mentioned
                    .iter()
                    .copied()
                    .find(|mode| *mode != preference.desired)
            })?;
        return Some(InteractionPreferenceAnswerFocusIR::ModeChoice {
            desired: preference.desired,
            rejected,
        });
    }
    (attributed_interaction_preference_predicate(question).as_deref() == Some("줄이다")
        && (preference.response_manner == Some(ResponseMannerIR::Concise)
            || preference.desired == InteractionModeIR::Concise))
        .then_some(InteractionPreferenceAnswerFocusIR::ResponseLength)
}

/// Operation + deictic event argument, not a whole-sentence alias. Consumes
/// the entire small request grammar; an unknown topic or a second operation
/// is not permission to bind the most recent event.
pub(crate) fn is_deictic_event_recap(text: &str) -> bool {
    let lower = text.trim().trim_end_matches(['.', '?', '!']).to_lowercase();
    let words = lower.split_whitespace().collect::<Vec<_>>();
    let mut remaining = words.as_slice();
    if matches!(
        remaining.first(),
        Some(&"would" | &"could" | &"can" | &"will")
    ) {
        if remaining.get(1) != Some(&"you") {
            return false;
        }
        remaining = &remaining[2..];
    }
    while matches!(remaining.first(), Some(&"please" | &"briefly" | &"just")) {
        remaining = &remaining[1..];
    }
    if matches!(remaining.first(), Some(&"summarize" | &"summarise")) {
        remaining = &remaining[1..];
        if remaining.first() == Some(&"briefly") {
            remaining = &remaining[1..];
        }
        if remaining.last() == Some(&"briefly") {
            remaining = &remaining[..remaining.len() - 1];
        }
        return match remaining {
            [] | ["that"] | ["it"] | ["what", "happened"] => true,
            [determiner, noun] => {
                matches!(*determiner, "the" | "that" | "this")
                    && matches!(*noun, "event" | "incident" | "occurrence")
            }
            _ => false,
        };
    }
    if matches!(remaining.first(), Some(&"방금" | &"아까")) {
        remaining = &remaining[1..];
    }
    if remaining.first() == Some(&"그") {
        remaining = &remaining[1..];
    }
    if matches!(
        remaining.first(),
        Some(&"일" | &"일을" | &"내용" | &"내용을")
    ) {
        remaining = &remaining[1..];
    }
    if matches!(remaining.first(), Some(&"간단히" | &"짧게")) {
        remaining = &remaining[1..];
    }
    matches!(
        remaining,
        ["요약해" | "요약해줘" | "요약해줄래" | "요약해줄래요" | "요약해주세요"]
    ) || (remaining.len() == 2
        && matches!(remaining[0], "요약" | "요약을")
        && crate::compositional_semantics::korean_nominal_request_tail(remaining[1]))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentProjectionIR {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elaboration_omitted_roles: Vec<ContentSlotIR>,
    /// Supporting detail is the original observed event, never an inferred
    /// cause or a replacement for the requested role/perspective.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elaboration_event: Option<DescribedEventIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_perspective: Option<EventPerspectiveProjectionIR>,
    /// Other source-bound answers to the same open role question. These are
    /// conjunctive answers, not alternative referent guesses. Nesting is banned.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub co_answers: Vec<ContentProjectionIR>,
    pub belief_id: String,
    pub source_actor: String,
    pub source_proposition: String,
    pub binding: ContentBindingIR,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_bindings: Vec<ContentBindingIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_sources: Vec<EventSourceIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_context: Option<EventReferenceContextIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reference_bindings: Vec<EventRoleReferenceIR>,
}

pub(crate) fn valid_elaboration_omissions(
    event: &DescribedEventIR,
    omitted: &[ContentSlotIR],
) -> bool {
    omitted.len() <= 3
        && omitted
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == omitted.len()
        && omitted.iter().all(|role| {
            matches!(
                role,
                ContentSlotIR::Location | ContentSlotIR::Time | ContentSlotIR::Duration
            ) && event.roles.contains_key(role)
        })
}

impl ContentProjectionIR {
    pub fn all_projections(&self) -> impl Iterator<Item = &Self> {
        std::iter::once(self).chain(&self.co_answers)
    }

    pub fn matches_question(&self, question: &str) -> bool {
        if self
            .co_answers
            .iter()
            .any(|p| !p.co_answers.is_empty() || !p.matches_question(question))
        {
            return false;
        }
        let Some(mut query) = described_event(question, true) else {
            return false;
        };
        if query.has_references() {
            let Some((resolved, bindings)) = self
                .reference_context
                .as_ref()
                .and_then(|c| c.resolve(&query))
            else {
                return false;
            };
            if bindings != self.reference_bindings {
                return false;
            }
            query = resolved;
        } else if self.reference_context.is_some() || !self.reference_bindings.is_empty() {
            return false;
        }
        let Some(content) = PropositionContentIR::compile_contextual(
            &self.source_proposition,
            &self.context_sources,
        ) else {
            return false;
        };
        requested_content_slots(question) == self.all_bindings().map(|b| b.slot).collect::<Vec<_>>()
            && content.events.iter().any(|e| {
                Some(&e.event_id) == self.binding.event_id.as_ref()
                    && e.query_view(&query).is_some_and(|(view, proof)| {
                        proof == self.event_perspective
                            && view.matches_direct(&query, self.binding.slot)
                    })
            })
    }

    pub(crate) fn bindings_grounded_in(&self, content: &PropositionContentIR) -> bool {
        if self
            .all_bindings()
            .any(|binding| self.elaboration_omitted_roles.contains(&binding.slot))
        {
            return false;
        }
        if !self.elaboration_omitted_roles.is_empty()
            && self.elaboration_event.as_ref().is_none_or(|event| {
                !valid_elaboration_omissions(event, &self.elaboration_omitted_roles)
            })
        {
            return false;
        }
        if self.elaboration_event.as_ref().is_some_and(|event| {
            Some(&event.event_id) != self.binding.event_id.as_ref()
                || !content.events.contains(event)
        }) {
            return false;
        }
        if let Some(proof) = &self.event_perspective {
            content
                .events
                .iter()
                .filter(|e| Some(&e.event_id) == self.binding.event_id.as_ref())
                .filter_map(|e| proof.apply(e))
                .any(|view| {
                    let bindings = view.bindings();
                    self.all_bindings()
                        .all(|b| b.event_id == self.binding.event_id && bindings.contains(b))
                })
        } else {
            self.all_bindings()
                .all(|b| b.event_id == self.binding.event_id && content.bindings.contains(b))
        }
    }

    pub fn validate(&self) -> bool {
        if !self.co_answers.is_empty()
            && (self.co_answers.len() > 7
                || self.binding.event_id.is_none()
                || !self.additional_bindings.is_empty()
                || self.co_answers.iter().any(|p| {
                    !p.co_answers.is_empty()
                        || !p.additional_bindings.is_empty()
                        || p.binding.event_id.is_none()
                        || p.binding.slot != self.binding.slot
                        || p.source_actor != self.source_actor
                        || !p.validate()
                })
                || self
                    .all_projections()
                    .map(|p| (&p.belief_id, &p.binding.event_id))
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != 1 + self.co_answers.len())
        {
            return false;
        }
        let Some(content) = PropositionContentIR::compile_contextual(
            &self.source_proposition,
            &self.context_sources,
        ) else {
            return false;
        };
        !self.belief_id.is_empty()
            && !self.source_actor.is_empty()
            && (self.reference_context.is_some() != self.reference_bindings.is_empty())
            && (self.reference_context.is_none() || self.binding.event_id.is_some())
            && self.reference_bindings.len() <= 7
            && self
                .reference_context
                .as_ref()
                .is_none_or(|c| c.event().is_some())
            && self.additional_bindings.len() <= 6
            && self.bindings_grounded_in(&content)
            && self
                .all_bindings()
                .map(|b| b.slot)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == 1 + self.additional_bindings.len()
    }

    pub fn all_bindings(&self) -> impl Iterator<Item = &ContentBindingIR> {
        std::iter::once(&self.binding).chain(&self.additional_bindings)
    }
}

#[cfg(test)]
mod event_tests {
    use super::*;

    #[test]
    fn addressed_question_owns_its_event_participants() {
        for question in [
            "Tell me who lent a telescope to Iona in detail.",
            "Tell me who sent a parcel to Teo in detail.",
            "Tell us who borrowed an umbrella from Nuri in detail.",
        ] {
            let request = question_request(question).expect(question);
            assert!(request.validate());
            assert!(described_event(&request.question_text, true).is_some());
            assert_eq!(request.response_manner, Some(ResponseMannerIR::Detailed));
        }
        for non_request in [
            "Tell Alice who lent a telescope to Iona in detail.",
            "Tell me who lent a telescope to Iona and delete the file.",
            "Do not tell me who lent a telescope to Iona.",
            "If you tell me who lent a telescope to Iona, wait.",
        ] {
            assert!(question_request(non_request).is_none(), "{non_request}");
        }
    }

    #[test]
    fn supplied_perspective_knowledge_has_bijective_source_linked_roles() {
        let knowledge = event_perspective_knowledge();
        let pack = crate::lexical_knowledge_pack::builtin_pack();
        let mut relations = std::collections::BTreeSet::new();
        for relation in &knowledge.relations {
            assert!(relations.insert(&relation.relation_id));
            let mut perspectives = std::collections::BTreeSet::new();
            for perspective in &relation.perspectives {
                assert!(perspectives.insert(&perspective.perspective_id));
                assert!(!perspective.lexical_entry_ids.is_empty());
                assert!(perspective
                    .lexical_entry_ids
                    .iter()
                    .all(|id| pack.entry(id).is_some()));
                assert_eq!(
                    perspective
                        .roles
                        .values()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len(),
                    perspective.roles.len()
                );
                assert!(relation
                    .required_participants
                    .iter()
                    .all(|p| perspective.roles.values().any(|v| v == p)));
            }
        }
    }

    #[test]
    fn event_perspectives_preserve_participants_without_changing_source() {
        for (report, question, expected) in [
            (
                "민수는 지연에게 책을 빌려줬어.",
                "누가 민수에게서 책을 빌렸어?",
                "지연",
            ),
            (
                "지연은 민수에게서 우산을 빌렸어.",
                "누가 지연에게 우산을 빌려줬어?",
                "민수",
            ),
            (
                "Mira lent a book to Noel.",
                "Who borrowed a book from Mira?",
                "Noel",
            ),
            (
                "Noel borrowed an umbrella from Mira.",
                "Who lent an umbrella to Noel?",
                "Mira",
            ),
        ] {
            let event = described_event(report, false).expect(report);
            let original = event.clone();
            let query = described_event(question, true).expect(question);
            let (view, proof) = event.query_view(&query).unwrap();
            assert!(proof.is_some());
            assert!(event.matches(&query, ContentSlotIR::Agent));
            assert_eq!(view.roles[&ContentSlotIR::Agent], expected);
            assert_eq!(event, original);
            let mut projection = ContentProjectionIR {
                elaboration_omitted_roles: vec![],
                elaboration_event: None,
                event_perspective: proof,
                co_answers: vec![],
                belief_id: "B1".into(),
                source_actor: "USER".into(),
                source_proposition: report.into(),
                binding: view
                    .bindings()
                    .into_iter()
                    .find(|b| b.slot == ContentSlotIR::Agent)
                    .unwrap(),
                additional_bindings: vec![],
                context_sources: vec![],
                reference_context: None,
                reference_bindings: vec![],
            };
            assert!(projection.validate());
            assert!(projection.matches_question(question));
            let mut no_relation = projection.clone();
            no_relation.event_perspective = None;
            assert!(!no_relation.validate());
            assert!(!no_relation.matches_question(question));
            let mut wrong_value = projection.clone();
            wrong_value.binding.value = "FABRICATED".into();
            assert!(!wrong_value.validate());
            let restored = view.query_view(&original).unwrap().0;
            assert_eq!(restored.roles, original.roles);
            projection.event_perspective.as_mut().unwrap().relation_id = "UNSUPPLIED".into();
            assert!(!projection.validate());
            assert!(!projection.matches_question(question));
        }
        for (report, question) in [
            (
                "Mira lent a book to Noel.",
                "Who borrowed a book from Noel?",
            ),
            (
                "Mira did not lend a book to Noel.",
                "Who borrowed a book from Mira?",
            ),
            ("Mira lent a book.", "Who borrowed a book?"),
            (
                "Mira sent a book to Noel.",
                "Who received a book from Mira?",
            ),
        ] {
            assert!(
                !described_event(report, false).unwrap().matches(
                    &described_event(question, true).unwrap(),
                    ContentSlotIR::Agent
                ),
                "{report} / {question}"
            );
        }
    }

    #[test]
    fn topic_question_scopes_events_without_inventing_a_role() {
        for (source, expected, slot) in [
            (
                "아까 무루 이야기로 돌아가서, 누가 읽었어?",
                "무루",
                ContentSlotIR::Agent,
            ),
            (
                "Back to Zorim, where did Mira read it?",
                "Zorim",
                ContentSlotIR::Location,
            ),
            (
                "아까 도서관 이야기로 돌아가서, 누가 읽었어?",
                "도서관",
                ContentSlotIR::Agent,
            ),
        ] {
            let query = described_event(source, true).expect(source);
            assert_eq!(query.topic_constraint.as_deref(), Some(expected));
            assert_eq!(requested_content_slot(source), Some(slot));
            assert!(!query.roles.values().any(|v| v == expected));
            assert!(described_event(source, false).is_none());
        }
        for source in [
            "Don't go back to Zorim, who read it?",
            "If we go back to Zorim, who read it?",
            "무루 이야기로 돌아가라고, 누가 읽었어?",
            "무루 이야기로 돌아가지 말고, 누가 읽었어?",
            "Back to Zorim, delete it.",
            "Back to Zorim, back to letter, who read it?",
        ] {
            assert!(
                crate::conversation::topic_question_parts(source).is_none(),
                "{source}"
            );
        }
        let report = described_event("서윤은 도서관에서 책을 읽었어.", false).unwrap();
        assert!(report.topic_constraint.is_none());
        assert!(report.matches(
            &described_event("아까 도서관 이야기로 돌아가서, 누가 읽었어?", true).unwrap(),
            ContentSlotIR::Agent
        ));
        assert_eq!(
            crate::conversation::detect_topic_transition("아까 이야기로 돌아가자.")
                .unwrap()
                .kind,
            crate::conversation::TopicTransitionKindIR::ReturnPrevious
        );
    }

    #[test]
    fn recap_request_composes_mood_operator_and_deictic_argument() {
        for mood in ["", "Please ", "Could you ", "Would you briefly "] {
            for argument in ["that", "it", "the event", "what happened"] {
                assert!(is_deictic_event_recap(&format!(
                    "{mood}summarize {argument}?"
                )));
            }
        }
        for reference in ["", "방금 ", "방금 그 일을 ", "아까 내용을 "] {
            for ending in ["요약해줘", "요약해줄래", "요약해주세요"] {
                assert!(is_deictic_event_recap(&format!("{reference}{ending}.")));
            }
        }
        for raw in [
            "Could she summarize that?",
            "Do not summarize that.",
            "Summarize the trial.",
            "Summarize that and delete it.",
            "What summarize that would event?",
            "그 내용을 요약하지 마.",
            "그 사람이 요약해줘.",
        ] {
            assert!(!is_deictic_event_recap(raw), "{raw}");
        }
    }

    #[test]
    fn request_mood_is_independent_of_question_surface_and_negation_scope() {
        for noun in ["조언", "해법", "해결책"] {
            for ending in ["들어줄래?", "들어줄래요?", "들어주시겠어요?"] {
                let raw = format!("지금은 {noun} 말고 내 얘기 좀 {ending}");
                let p = interaction_preference(&raw).expect("addressed request");
                assert_eq!(p.desired, InteractionModeIR::Listening, "{raw}");
                assert_eq!(p.mood, InteractionMoodIR::InterrogativeRequest);
                assert_eq!(p.excluded, vec![InteractionModeIR::Advice]);
                assert!(p.owns_response());
                assert!(PropositionContentIR::compile(&raw).validate_source(&raw));
            }
        }
        for modal in ["Can", "Could", "Would", "Will"] {
            let raw = format!("{modal} you listen to me for a moment?");
            let p = interaction_preference(&raw).unwrap();
            assert_eq!(p.mood, InteractionMoodIR::InterrogativeRequest);
            assert!(p.owns_response());
        }
        let p =
            interaction_preference("You don't have to solve anything. Just stay and chat with me.")
                .unwrap();
        assert_eq!(p.desired, InteractionModeIR::Conversation);
        assert_eq!(p.excluded, vec![InteractionModeIR::Advice]);
        assert_eq!(p.mood, InteractionMoodIR::Directive);
        assert!(p.owns_response());
        let p = interaction_preference("짧게만 말하지 말고, 이번엔 이유도 설명해줘.").unwrap();
        assert_eq!(p.desired, InteractionModeIR::Explanation);
        assert_eq!(p.excluded, vec![InteractionModeIR::Concise]);
        assert!(!p.owns_response());
        assert!(
            interaction_preference("요약해달라는 건 아니고, 잠깐 대화하자.")
                .unwrap()
                .owns_response()
        );
        for raw in [
            "Do I want a solution right now?",
            "Can she listen to me?",
            "Who will listen to me?",
            "누가 내 얘기를 들어줄래?",
            "민수는 내 얘기를 들어줄래?",
            "I don't want to chat.",
            "Could you not listen to me?",
            "왜 내 얘기를 들어줄래?",
            "If I ask, could you listen?",
            "Is it late? Could you listen?",
        ] {
            assert!(
                interaction_preference(raw).is_none(),
                "inquiry/negative: {raw}"
            );
        }
        for raw in [
            "Could you chat and delete the file?",
            "Just chat with me and modify the document.",
            "얘기하자. 파일을 삭제해.",
            "Could you chat with me and erase the backup?",
            "내 얘기를 듣고 보관함을 비워줄래?",
        ] {
            assert!(
                !interaction_preference(raw).is_some_and(|p| p.owns_response()),
                "mixed action: {raw}"
            );
        }
    }

    #[test]
    fn interaction_preferences_preserve_positive_and_negative_clause_scope() {
        for text in [
            "비가 오면 내 얘기를 들어줬으면 해.",
            "민수는 내 얘기를 들어줬으면 해.",
            "내 얘기를 안 들어줬으면 해.",
        ] {
            assert!(
                interaction_preference(text).is_none_or(|p| !p.owns_response()),
                "{text}"
            );
        }
        for excluded in ["조언", "해결책"] {
            for wanted in ["대화하고 싶어", "얘기하자", "이야기하고 싶어요"] {
                let raw = format!("{excluded}은 필요 없어, 그냥 {wanted}.");
                let p = interaction_preference(&raw).expect("preference");
                assert_eq!(p.desired, InteractionModeIR::Conversation);
                assert!(p.excluded.contains(&InteractionModeIR::Advice));
                assert!(p.owns_response());
                assert!(PropositionContentIR::compile(&raw).validate_source(&raw));
            }
        }
        for raw in [
            "I am not looking for advice; I could use someone to listen.",
            "오늘은 조언보다 내 얘기를 들어주는 사람이 있었으면 좋겠네.",
        ] {
            let p = interaction_preference(raw).expect("indirect desire");
            assert_eq!(p.desired, InteractionModeIR::Listening);
            assert!(p.owns_response());
        }
        assert!(!interaction_preference("얘기하자. 문서를 삭제해.")
            .unwrap()
            .owns_response());
        assert!(
            !interaction_preference("I want to chat and delete the document.")
                .unwrap()
                .owns_response()
        );
        let explanation = interaction_preference("설명만 듣고 싶어.").unwrap();
        assert_eq!(explanation.desired, InteractionModeIR::Explanation);
        assert!(!explanation.owns_response());
        let nominal = PropositionContentIR::compile("내가 원하는 건 짧은 대답이야.");
        let intention = nominal
            .bindings
            .iter()
            .find(|binding| binding.slot == ContentSlotIR::Intention)
            .unwrap();
        assert_eq!(intention.value, "짧은 대답");
        assert_eq!(intention.grammar_evidence, "DESIDERATIVE_NOMINAL");
        let listening =
            interaction_preference("오늘은 조언보다 내 얘기를 들어주는 사람이 있었으면 좋겠네.")
                .unwrap();
        assert_eq!(
            interaction_preference_answer_focus(
                "내가 지금 바라는 건 해결책일까, 들어주는 걸까?",
                &listening,
            ),
            Some(InteractionPreferenceAnswerFocusIR::ModeChoice {
                desired: InteractionModeIR::Listening,
                rejected: InteractionModeIR::Advice,
            })
        );
        let concise = interaction_preference("내가 원하는 건 짧은 대답이야.").unwrap();
        assert_eq!(
            interaction_preference_answer_focus("그러니까 지금은 뭘 줄여 달라는 거야?", &concise,),
            Some(InteractionPreferenceAnswerFocusIR::ResponseLength)
        );
        assert_eq!(
            interaction_preference_answer_focus("내가 원하는 게 뭐야?", &listening),
            None
        );
        for question in [
            "그러니까 지금은 뭘 줄여 달라는 거야?",
            "내가 무엇을 바꾸어 달라는 뜻이야?",
            "제가 뭐를 설명해 달라는 거예요?",
        ] {
            assert!(
                attributed_interaction_preference_query(question),
                "{question}"
            );
            assert_eq!(
                requested_content_slot(question),
                Some(ContentSlotIR::Intention)
            );
        }
        for question in [
            "민수가 뭘 줄여 달라는 거야?",
            "그러니까 지금은 뭘 고무 달라는 거야?",
            "그러니까 지금은 뭘 줄여 달라는 거야.",
        ] {
            assert!(
                !attributed_interaction_preference_query(question),
                "{question}"
            );
        }
        assert!(interaction_preference("준서는 대화하고 싶어.").is_none());
        assert!(interaction_preference("I don't want to talk.").is_none());
        assert!(interaction_preference("If I want to talk, will you listen?").is_none());
        let raw = "I want to chat.";
        let mut content = PropositionContentIR::compile(raw);
        content.interaction_preference.as_mut().unwrap().desired = InteractionModeIR::Execution;
        assert!(!content.validate_source(raw));
    }

    #[test]
    fn role_gap_grammar_composes_heads_negation_and_voice() {
        let prior = described_event("하영은 강당에서 지도를 읽었어.", false).unwrap();
        let revised = compose_report("강당이 아니고 교실이야.", Some(&prior)).unwrap();
        assert_eq!(revised.roles[&ContentSlotIR::Location], "교실");
        assert_eq!(revised.roles[&ContentSlotIR::Agent], "하영");
        assert!(compose_report("식당이 아니고 교실이야.", Some(&prior)).is_none());
        assert!(compose_report("강당이 아니고 교실이야.", None).is_none());
        for noun in ["계약서", "지도", "일기"] {
            for head in [
                "사람은",
                "사람을 알려줘",
                "장소도 알려줄래",
                "장소랑 사람을 같이 알려줘",
            ] {
                let source = format!("{noun}를 읽은 {head}?");
                assert!(is_event_question(&source), "{source}");
                let event = described_event(&source, true).expect("role gap");
                assert_eq!(event.roles[&ContentSlotIR::Theme], noun);
            }
            let q = described_event(&format!("{noun}를 읽지 않은 사람은 누구야?"), true).unwrap();
            assert!(q.negated);
            assert!(
                !described_event(&format!("누가 {noun}를 읽었어?"), true)
                    .unwrap()
                    .negated
            );
        }
        assert_eq!(
            requested_content_slots("읽은 장소랑 사람을 같이 알려줘."),
            vec![ContentSlotIR::Location, ContentSlotIR::Agent]
        );
        let contextual_location = described_event("읽은 장소도 알려줄래?", true).unwrap();
        assert_eq!(
            requested_content_slots("읽은 장소도 알려줄래?"),
            vec![ContentSlotIR::Location]
        );
        assert!(contextual_location.roles.is_empty());
        assert!(!contextual_location.lexical_entry_ids.is_empty());
        assert!(!predicate_entries("빌려").is_empty());
        assert!(crate::lexical_knowledge_pack::builtin_pack()
            .lookup("줄여")
            .matches
            .iter()
            .any(|matched| matched.entry.lemma == "줄이다"));
        let mut auxiliary_context = vec!["빌려"];
        assert!(korean_relative_directional_principal(&mut auxiliary_context, "간").is_some());
        let borrower =
            described_event("아까 신문 얘기로 돌아가서, 빌려 간 사람 누구였지?", true).unwrap();
        assert!(borrower.roles.is_empty());
        assert_eq!(borrower.predicate_surface, "빌려 간");
        assert!(borrower.lexical_entry_ids.contains(&"17824".to_string()));
        for verb in ["read", "lent"] {
            let active =
                described_event(&format!("alex {verb} a document at the park"), false).unwrap();
            let passive =
                described_event(&format!("a document was {verb} by alex at the park"), false)
                    .unwrap();
            assert_eq!(active.roles, passive.roles);
            assert_eq!(active.lexical_entry_ids, passive.lexical_entry_ids);
        }
        let q = described_event("Which person didn't read it?", true).unwrap();
        assert!(q.negated && q.has_references());
        let q = described_event("And who read it, again?", true).unwrap();
        assert!(!q.negated && q.has_references());
        for command in [
            "문서를 읽고 삭제해",
            "read it and delete it",
            "send the document",
        ] {
            assert!(
                !is_event_question(command),
                "must not consume action: {command}"
            );
        }
    }

    #[test]
    fn property_question_grammar_keeps_owner_separate_from_information_head() {
        for (bearer, questions) in [
            (
                "수빈",
                vec![
                    "수빈은 어때?",
                    "수빈은 어떤 상태야",
                    "수빈의 상태는 어때?",
                    "수빈의 상태가 무엇인가요?",
                ],
            ),
            (
                "소연의 친구",
                vec![
                    "소연의 친구는 어떤 상태인가요?",
                    "소연의 친구의 상태는 무엇이야?",
                ],
            ),
            (
                "the visitor",
                vec![
                    "How is the visitor?",
                    "What is the state of the visitor?",
                    "How is the condition of the visitor?",
                ],
            ),
        ] {
            for text in questions {
                let q = described_state(text, true).unwrap_or_else(|| panic!("{text}"));
                assert_eq!(q.roles[&ContentSlotIR::Theme], bearer, "{text}");
                assert_eq!(q.kind, DescriptionKindIR::State);
                assert!(q.lexical_entry_ids.is_empty());
                assert_eq!(requested_content_slots(text), vec![ContentSlotIR::Property]);
                assert!(is_event_question(text), "{text}");
                assert!(
                    described_state(text, false).is_none(),
                    "question is not a fact: {text}"
                );
            }
        }
        for text in [
            "수빈은 어떤 색이야?",
            "수빈은 어떤 상태가 아니야?",
            "수빈의 상태는 몰라.",
            "수빈은 어떤 상태야? 파일을 삭제해.",
            "누가 어떤 상태야?",
            "What is the state of the visitor and the teacher?",
            "What is the color of the visitor?",
            "\"수빈은 어떤 상태야?\"",
        ] {
            assert!(
                described_state(text, true).is_none(),
                "unconsumed or different relation: {text}"
            );
        }
    }

    #[test]
    fn role_questions_support_nominal_passive_and_multiple_gaps() {
        for noun in ["잡지", "편지", "문서"] {
            for predicate in ["읽은", "빌려준"] {
                let q =
                    described_event(&format!("{noun}를 {predicate} 사람은 누구야?"), true).unwrap();
                assert_eq!(q.roles[&ContentSlotIR::Theme], noun);
                assert!(!q.lexical_entry_ids.is_empty());
            }
        }
        let q = described_event("Who was the letter lent to?", true).unwrap();
        assert_eq!(q.roles[&ContentSlotIR::Theme], "letter");
        assert_eq!(
            requested_content_slot("그럼 받은 사람은?"),
            Some(ContentSlotIR::Recipient)
        );
        assert_eq!(requested_content_slots("누가 어디서 읽었어?").len(), 2);
    }

    #[test]
    fn contextual_reports_replay_references_and_corrections() {
        let first = EventSourceIR {
            belief_id: "B1".into(),
            source_actor: "DIALOGUE_USER".into(),
            source_proposition: "수아는 유진에게 문서를 빌려줬어.".into(),
        };
        let second = "유진은 서점에서 그것을 읽었어.";
        let c =
            PropositionContentIR::compile_contextual(second, std::slice::from_ref(&first)).unwrap();
        assert_eq!(c.events[0].roles[&ContentSlotIR::Theme], "문서");
        assert!(c.validate_source(second));
        let mut chain = vec![first];
        chain.push(EventSourceIR {
            belief_id: "B2".into(),
            source_actor: "DIALOGUE_USER".into(),
            source_proposition: second.into(),
        });
        let correction = "아니, 서점이 아니라 학교에서 읽었어.";
        let c = PropositionContentIR::compile_contextual(correction, &chain).unwrap();
        assert_eq!(c.events[0].roles[&ContentSlotIR::Theme], "문서");
        assert_eq!(c.events[0].roles[&ContentSlotIR::Location], "학교");
        assert!(c.validate_source(correction));
        let mut tampered = c.clone();
        tampered.events[0]
            .roles
            .insert(ContentSlotIR::Agent, "몰래".into());
        assert!(!tampered.validate_source(correction));
        assert!(PropositionContentIR::compile_contextual(
            "아니, 공원이 아니라 학교에서 읽었어.",
            &chain
        )
        .is_none());
    }

    #[test]
    fn reference_role_resolution_transfers_without_sentence_templates() {
        for actor in ["소라", "미리", "루나"] {
            for theme in ["지도", "잡지", "편지"] {
                let source = format!("{actor}는 서점에서 {theme}를 읽었어.");
                let event = described_event(&source, false).unwrap();
                let context = EventReferenceContextIR {
                    belief_id: "SOURCE-REPORT".into(),
                    source_actor: "DIALOGUE_USER".into(),
                    source_proposition: source,
                    event_id: event.event_id,
                    focused_slot: None,
                    context_sources: vec![],
                };
                for mention in ["그것".to_string(), format!("그 {theme}")] {
                    let question =
                        described_event(&format!("누가 {mention}을 읽었어?"), true).unwrap();
                    let (resolved, bindings) = context.resolve(&question).unwrap();
                    assert_eq!(resolved.roles[&ContentSlotIR::Theme], theme);
                    assert_eq!(bindings.len(), 1);
                    assert_eq!(bindings[0].antecedent_slot, ContentSlotIR::Theme);
                }
                let query = described_event("누가 그 우산을 읽었어?", true).unwrap();
                assert!(context.resolve(&query).is_none());
            }
        }
    }

    #[test]
    fn event_role_grammar_transfers_across_a_generated_entity_order_grid() {
        for actor in ["라미", "수아", "미리"] {
            for theme in ["지도", "편지", "문서"] {
                for predicate in ["읽었어", "빌려줬어"] {
                    for text in [
                        format!("{actor}는 {theme}를 {predicate}."),
                        format!("{theme}를 {actor}는 {predicate}."),
                    ] {
                        let event = described_event(&text, false).unwrap();
                        let question =
                            described_event(&format!("누가 {theme}를 {predicate}?"), true).unwrap();
                        assert!(event.matches(&question, ContentSlotIR::Agent));
                        assert_eq!(event.roles[&ContentSlotIR::Agent], actor);
                        assert_eq!(event.roles[&ContentSlotIR::Theme], theme);
                    }
                }
            }
        }
    }

    #[test]
    fn event_roles_are_compositional_and_source_bound() {
        for text in [
            "어제 민수는 지연에게 책을 빌려줬어.",
            "책을 지연에게 어제 민수가 빌려줬어.",
        ] {
            let content = PropositionContentIR::compile(text);
            let event = content.events.first().expect(text);
            assert_eq!(event.roles[&ContentSlotIR::Agent], "민수");
            assert_eq!(event.roles[&ContentSlotIR::Recipient], "지연");
            assert_eq!(event.roles[&ContentSlotIR::Theme], "책");
            assert_eq!(event.roles[&ContentSlotIR::Time], "어제");
            assert!(content.validate_source(text));
            let mut tampered = content.clone();
            tampered.events[0]
                .roles
                .insert(ContentSlotIR::Agent, "다른 사람".into());
            assert!(!tampered.validate_source(text));
            assert!(!event
                .bindings()
                .iter()
                .any(|b| b.slot == ContentSlotIR::Cause));
        }
    }

    #[test]
    fn event_predicates_share_bilingual_lexical_identity_not_effects() {
        let ko = described_event("민수는 지연에게 책을 빌려줬어.", false).unwrap();
        let en = described_event("민수 lent a book to 지연.", false);
        // A mixed-script sentence is outside the bounded grammar; language
        // identity is nevertheless shared by predicate entries.
        assert!(en.is_none());
        let en = described_event("Minsu lent a book to Jiyeon yesterday.", false).unwrap();
        assert!(ko
            .lexical_entry_ids
            .iter()
            .any(|id| en.lexical_entry_ids.contains(id)));
        for (slot, value) in [
            (ContentSlotIR::Agent, "Minsu"),
            (ContentSlotIR::Recipient, "Jiyeon"),
            (ContentSlotIR::Theme, "book"),
            (ContentSlotIR::Time, "yesterday"),
        ] {
            assert_eq!(en.roles[&slot], value);
        }
        let question = described_event("To whom did Minsu lend a book?", true).unwrap();
        assert!(en.matches(&question, ContentSlotIR::Recipient));
        assert!(!en.matches(
            &described_event("To whom did Mina lend a book?", true).unwrap(),
            ContentSlotIR::Recipient
        ));
    }

    #[test]
    fn event_grammar_abstains_on_unconsumed_or_embedded_content() {
        for text in [
            "민수가 책을 읽고",
            "민수가 민수는 책을 읽었어.",
            "민수가 책을 읽었어. 지연은 잤어.",
            "민수가 책을 읽었다고 말했어.",
            "Mina lent a book to",
            "Mina lent a book to from Jin.",
            "If Mina lent a book to Jin.",
            "Mina lent a book and Jin slept.",
        ] {
            assert!(described_event(text, false).is_none(), "{text}");
        }
        let neg = described_event("민수가 책을 안 읽었어.", false).unwrap();
        assert!(neg.negated);
        assert!(!neg.matches(
            &described_event("누가 책을 읽었어?", true).unwrap(),
            ContentSlotIR::Agent
        ));
        assert!(neg.matches(
            &described_event("누가 책을 안 읽었어?", true).unwrap(),
            ContentSlotIR::Agent
        ));
    }
}
