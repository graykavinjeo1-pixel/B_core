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

/// Explicitly supplied affordance knowledge. Labels are realization metadata;
/// the core receives only state/action literals and a possible-effect relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionBenefitPrimitiveIR {
    pub id: String,
    pub authorship: String,
    pub state_property: WorldPropertyIR,
    pub state_value: bool,
    pub possible_effect_id: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionBenefitExpressionIR {
    pub primitive_id: String,
    pub action_ko: String,
    pub action_en: String,
    pub effect_ko: String,
    pub effect_en: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ActionBenefitBindingIR {
    primitive_id: String,
    entry_id: String,
    sense_id: String,
    source_ref: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ActionBenefitLanguageIR {
    bindings: Vec<ActionBenefitBindingIR>,
    expressions: Vec<ActionBenefitExpressionIR>,
}

struct ActionBenefitKnowledge {
    primitives: BTreeMap<String, ActionBenefitPrimitiveIR>,
    by_sense: BTreeMap<(String, String), BTreeSet<String>>,
    expressions: BTreeMap<String, ActionBenefitExpressionIR>,
}

impl ActionBenefitKnowledge {
    fn new(
        primitives: Vec<ActionBenefitPrimitiveIR>,
        language: ActionBenefitLanguageIR,
    ) -> Result<Self, &'static str> {
        if primitives.is_empty()
            || primitives.len() > 32
            || language.bindings.len() > 256
            || language.expressions.len() > 32
        {
            return Err("ACTION_BENEFIT_KNOWLEDGE_CAPACITY");
        }
        let mut result = Self {
            primitives: BTreeMap::new(),
            by_sense: BTreeMap::new(),
            expressions: BTreeMap::new(),
        };
        for p in primitives {
            if p.id.is_empty() || result.primitives.insert(p.id.clone(), p).is_some() {
                return Err("DUPLICATE_ACTION_BENEFIT_PRIMITIVE");
            }
        }
        for b in language.bindings {
            if b.source_ref.is_empty()
                || !result.primitives.contains_key(&b.primitive_id)
                || !crate::lexical_knowledge_pack::builtin_pack()
                    .entry(&b.entry_id)
                    .is_some_and(|e| e.senses.iter().any(|s| s.source_sense_id == b.sense_id))
            {
                return Err("UNRESOLVED_ACTION_BENEFIT_BINDING");
            }
            if !result
                .by_sense
                .entry((b.entry_id, b.sense_id))
                .or_default()
                .insert(b.primitive_id)
            {
                return Err("DUPLICATE_ACTION_BENEFIT_BINDING");
            }
        }
        for e in language.expressions {
            if !result.primitives.contains_key(&e.primitive_id)
                || [&e.action_ko, &e.action_en, &e.effect_ko, &e.effect_en]
                    .iter()
                    .any(|s| s.trim().is_empty() || s.chars().count() > 256)
                || result
                    .expressions
                    .insert(e.primitive_id.clone(), e)
                    .is_some()
            {
                return Err("INVALID_ACTION_BENEFIT_EXPRESSION");
            }
        }
        Ok(result)
    }

    fn candidates(&self, action: &crate::utterance_intent::ProposedActionIR) -> BTreeSet<String> {
        use crate::proposition_content::ContentSlotIR as R;
        action
            .frame_candidates
            .iter()
            .filter(|f| {
                f.pattern_understood
                    && f.incompatible_roles.is_empty()
                    && f.missing_roles.iter().all(|r| *r == R::Agent)
            })
            .filter_map(|f| self.by_sense.get(&(f.entry_id.clone(), f.sense_id.clone())))
            .flatten()
            .cloned()
            .collect()
    }
}

fn action_benefit_knowledge() -> &'static ActionBenefitKnowledge {
    static DATA: std::sync::OnceLock<ActionBenefitKnowledge> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let source = include_str!("../data/lexical-knowledge/action-benefit-primitives.json");
        assert_eq!(
            format!("{:x}", Sha256::digest(source.as_bytes())),
            "8012f8543d0ea33b9e416b0a645230452168cf0d6ad4eb7bd1469cb6718b037c"
        );
        let data: Vec<ActionBenefitPrimitiveIR> =
            serde_json::from_str(source).expect("supplied action benefit primitives");
        let language = include_str!("../data/lexical-knowledge/action-benefit-language.json");
        assert_eq!(
            format!("{:x}", Sha256::digest(language.as_bytes())),
            "31aa7638c935df756ac5ea710aae7b9950048503c78d947cacd5d83ff3d8ea87"
        );
        ActionBenefitKnowledge::new(
            data,
            serde_json::from_str(language).expect("supplied action benefit language"),
        )
        .expect("consistent action benefit knowledge")
    })
}

/// A conditional interpretation and possible benefit, never a recommendation
/// to execute, a guaranteed transition, or a fact written back into the world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBenefitAssessmentIR {
    pub basis: Box<DialogueWorldIR>,
    pub evaluated_turn: u64,
    pub actor: String,
    pub primitive_id: String,
    pub request: DeliberationRequestIR,
    pub derivation: DeliberationIR,
    #[serde(default)]
    pub current_clarification: bool,
}

impl ActionBenefitAssessmentIR {
    pub(crate) fn primitive(&self) -> Option<&'static ActionBenefitPrimitiveIR> {
        action_benefit_knowledge()
            .primitives
            .get(&self.primitive_id)
    }

    pub(crate) fn expression(&self) -> Option<&'static ActionBenefitExpressionIR> {
        action_benefit_knowledge()
            .expressions
            .get(&self.primitive_id)
    }

    pub(crate) fn validate(
        &self,
        action: &crate::utterance_intent::ProposedActionIR,
        continues_context: bool,
    ) -> bool {
        assess_action_benefit_scoped(
            action,
            continues_context,
            &self.basis,
            self.evaluated_turn,
            self.current_clarification,
        )
        .as_ref()
            == Some(self)
    }

    pub(crate) fn matches_world(&self, world: &DialogueWorldIR) -> bool {
        // A later correction/conflict invalidates an old assessment; social or
        // explanatory focus changes cannot manufacture or erase its evidence.
        self.basis.premises == world.premises
            && self.basis.vocabulary == world.vocabulary
            && self.basis.implications == world.implications
    }
}

/// Why the available evidence cannot yet support an action judgement. These
/// are epistemic diagnostics, not claims that an action cannot help in reality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionBenefitGapReasonIR {
    InvalidContext,
    NegatedActionEffect,
    ActionRoles,
    Actor,
    EffectKnowledge,
    CurrentState,
    ConflictingState,
    InapplicableState,
    RecentState,
    AmbiguousEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBenefitGapIR {
    pub basis: Box<DialogueWorldIR>,
    pub evaluated_turn: u64,
    pub reason: ActionBenefitGapReasonIR,
    #[serde(default)]
    pub current_clarification: bool,
}

impl ActionBenefitGapIR {
    /// Ask for one precise missing atom, never a label-derived guess. The
    /// existing core must independently select this atom as its observation
    /// question. Positive wording avoids cross-language negative-question
    /// polarity conventions when binding a short yes/no answer.
    pub(crate) fn state_question(
        &self,
        action: &crate::utterance_intent::ProposedActionIR,
        continues_context: bool,
    ) -> Option<WorldQueryIR> {
        if self.reason != ActionBenefitGapReasonIR::CurrentState || self.current_clarification {
            return None;
        }
        let actor = action_benefit_actor(
            action,
            continues_context,
            &self.basis,
            self.evaluated_turn,
            false,
        )
        .ok()?;
        let knowledge = action_benefit_knowledge();
        let atoms = knowledge
            .candidates(action)
            .iter()
            .filter_map(|id| knowledge.primitives.get(id))
            .map(|p| WorldAtomIR {
                entity: actor.clone(),
                property: p.state_property.clone(),
                object: None,
            })
            .collect::<BTreeSet<_>>();
        if atoms.len() != 1 {
            return None;
        }
        let atom = atoms.into_iter().next()?;
        // A pending polar binding must never be installed if either supported
        // language cannot actually realize the question from this vocabulary.
        if matches!(atom.property, WorldPropertyIR::Registered(_))
            && [
                crate::language_knowledge::LanguageCodeIR::Korean,
                crate::language_knowledge::LanguageCodeIR::English,
            ]
            .iter()
            .any(|lang| {
                self.basis
                    .vocabulary
                    .expression(&atom.property, *lang)
                    .is_none()
            })
        {
            return None;
        }
        let query = WorldQueryIR {
            clarification_goal: Some(format!(
                "ACTION_BENEFIT:{:x}",
                Sha256::digest(
                    serde_json::to_vec(&(
                        &action.predicate_entry_ids,
                        action.negated,
                        &action.event.roles
                    ))
                    .ok()?
                )
            )),
            target: (atom.clone(), true),
            explain: false,
            assumption: None,
        };
        let world = deliberate_world(&self.basis, &query).ok()?;
        world
            .decision
            .question
            .as_ref()
            .filter(|q| q.proposition_id == atom.id())
            .map(|_| query)
    }

    pub(crate) fn validate(
        &self,
        action: &crate::utterance_intent::ProposedActionIR,
        continues_context: bool,
    ) -> bool {
        self.basis.validate(self.evaluated_turn)
            && action_benefit_basis(
                action,
                continues_context,
                &self.basis,
                self.evaluated_turn,
                self.current_clarification,
            )
            .err()
                == Some(self.reason)
    }

    pub(crate) fn matches_world(&self, world: &DialogueWorldIR) -> bool {
        self.basis.premises == world.premises
            && self.basis.vocabulary == world.vocabulary
            && self.basis.implications == world.implications
    }
}

pub(crate) fn action_benefit_gap(
    action: &crate::utterance_intent::ProposedActionIR,
    continues_context: bool,
    memory: &DialogueWorldIR,
    turn: u64,
) -> Option<ActionBenefitGapIR> {
    action_benefit_gap_scoped(action, continues_context, memory, turn, false)
}

pub(crate) fn action_benefit_gap_scoped(
    action: &crate::utterance_intent::ProposedActionIR,
    continues_context: bool,
    memory: &DialogueWorldIR,
    turn: u64,
    current_clarification: bool,
) -> Option<ActionBenefitGapIR> {
    if !memory.validate(turn) {
        return None;
    }
    Some(ActionBenefitGapIR {
        basis: Box::new(memory.clone()),
        evaluated_turn: turn,
        reason: action_benefit_basis(
            action,
            continues_context,
            memory,
            turn,
            current_clarification,
        )
        .err()?,
        current_clarification,
    })
}

/// A pending semantic question and a source-verified fact that fills its state
/// slot. It cannot authorize actions or resume an unrelated/new question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBenefitResumptionIR {
    pub original_source: String,
    pub question_turn: u64,
    pub prior_gap: Box<ActionBenefitGapIR>,
    pub update: Box<WorldMemoryUpdateIR>,
}

impl ActionBenefitResumptionIR {
    pub(crate) fn validate(&self) -> bool {
        use ActionBenefitGapReasonIR as G;
        let Some(original) = crate::utterance_intent::decision_inquiry(&self.original_source)
        else {
            return false;
        };
        let Some(action) = &original.proposed_action else {
            return false;
        };
        if self.prior_gap.current_clarification
            || !matches!(
                self.prior_gap.reason,
                G::CurrentState | G::RecentState | G::ConflictingState
            )
            || self.question_turn == 0
            || self.question_turn != self.prior_gap.evaluated_turn
            || self.update.turn <= self.question_turn
            || self.update.turn - self.question_turn > 3
            || !self.prior_gap.validate(action, original.continues_context)
            || !self.update.validate()
        {
            return false;
        }
        let mut prefix = (*self.prior_gap.basis).clone();
        if let Some(query) = self
            .prior_gap
            .state_question(action, original.continues_context)
        {
            prefix.last_query = Some(query);
        }
        let Ok(replay) = prefix.prepare(&self.update.source_text, self.update.turn) else {
            return false;
        };
        if !replay.recognized
            || replay
                .query
                .as_ref()
                .is_some_and(|q| Some(q) != prefix.last_query.as_ref())
            || replay.clarification.is_some()
            || replay.memory.premises != self.update.memory.premises
            || replay.memory.vocabulary != self.update.memory.vocabulary
            || replay.memory.implications != self.update.memory.implications
        {
            return false;
        }
        let Ok(actor) = action_benefit_actor(
            action,
            original.continues_context,
            &self.prior_gap.basis,
            self.question_turn,
            false,
        ) else {
            return false;
        };
        let knowledge = action_benefit_knowledge();
        knowledge
            .candidates(action)
            .iter()
            .filter_map(|id| knowledge.primitives.get(id))
            .any(|primitive| {
                self.update.memory.premises.iter().any(|p| {
                    p.active
                        && p.introduced_turn == self.update.turn
                        && p.source_text == self.update.source_text
                        && p.atom.entity == actor
                        && p.atom.property == primitive.state_property
                        && p.atom.object.is_none()
                })
            })
    }
}

fn action_benefit_actor(
    action: &crate::utterance_intent::ProposedActionIR,
    continues_context: bool,
    memory: &DialogueWorldIR,
    turn: u64,
    current_clarification: bool,
) -> Result<String, ActionBenefitGapReasonIR> {
    use crate::proposition_content::ContentSlotIR as R;
    use ActionBenefitGapReasonIR as G;
    if let Some(actor) = action.event.roles.get(&R::Agent) {
        Ok(match actor.to_lowercase().as_str() {
            "i" | "me" | "나" | "저" => "__user__".into(),
            _ => actor.to_lowercase(),
        })
    } else {
        let (focus, _) = memory.discourse.focus.as_ref().ok_or(G::Actor)?;
        if !continues_context
            || focus.entity != "__user__"
            || memory.discourse.source_turn > turn
            || (!current_clarification && memory.discourse.source_turn == turn)
            || turn - memory.discourse.source_turn > 3
        {
            return Err(G::Actor);
        }
        Ok(focus.entity.clone())
    }
}

fn action_benefit_basis<'a>(
    action: &crate::utterance_intent::ProposedActionIR,
    continues_context: bool,
    memory: &'a DialogueWorldIR,
    turn: u64,
    current_clarification: bool,
) -> Result<
    (
        String,
        &'static ActionBenefitPrimitiveIR,
        &'a WorldPremiseIR,
    ),
    ActionBenefitGapReasonIR,
> {
    use crate::proposition_content::ContentSlotIR as R;
    use ActionBenefitGapReasonIR as G;
    if !memory.validate(turn) {
        return Err(G::InvalidContext);
    }
    if action.negated {
        return Err(G::NegatedActionEffect);
    }
    if action.event.roles.keys().any(|r| *r != R::Agent) {
        return Err(G::ActionRoles);
    }
    let actor = action_benefit_actor(
        action,
        continues_context,
        memory,
        turn,
        current_clarification,
    )?;
    let knowledge = action_benefit_knowledge();
    // Multiple lexical paths to one primitive are one semantic alternative.
    // Distinct viable primitives still compete; aliases do not create votes.
    let candidates = knowledge.candidates(action);
    if candidates.is_empty() {
        return Err(G::EffectKnowledge);
    }
    let mut viable = Vec::new();
    let mut failures = Vec::new();
    for id in &candidates {
        let primitive = knowledge.primitives.get(id).ok_or(G::EffectKnowledge)?;
        let atom = WorldAtomIR {
            entity: actor.clone(),
            property: primitive.state_property.clone(),
            object: None,
        };
        let relevant = memory
            .premises
            .iter()
            .filter(|p| p.active && p.atom == atom)
            .collect::<Vec<_>>();
        if relevant.iter().any(|p| p.value != primitive.state_value) {
            failures.push(
                if relevant.iter().any(|p| p.value == primitive.state_value) {
                    G::ConflictingState
                } else {
                    G::InapplicableState
                },
            );
            continue;
        }
        let Some(premise) = relevant.last() else {
            failures.push(G::CurrentState);
            continue;
        };
        // Fresh source premises only; a prior assessment never feeds itself.
        if premise.introduced_turn > turn
            || (!current_clarification && premise.introduced_turn == turn)
            || turn - premise.introduced_turn > 3
        {
            failures.push(G::RecentState);
            continue;
        }
        viable.push((primitive, *premise));
    }
    match viable.as_slice() {
        [(primitive, premise)] => Ok((actor, *primitive, *premise)),
        [] if !failures.is_empty() && failures.iter().all(|r| *r == failures[0]) => {
            Err(failures[0])
        }
        _ => Err(G::AmbiguousEffect),
    }
}

pub(crate) fn assess_action_benefit(
    action: &crate::utterance_intent::ProposedActionIR,
    continues_context: bool,
    memory: &DialogueWorldIR,
    turn: u64,
) -> Option<ActionBenefitAssessmentIR> {
    assess_action_benefit_scoped(action, continues_context, memory, turn, false)
}

pub(crate) fn assess_action_benefit_scoped(
    action: &crate::utterance_intent::ProposedActionIR,
    continues_context: bool,
    memory: &DialogueWorldIR,
    turn: u64,
    current_clarification: bool,
) -> Option<ActionBenefitAssessmentIR> {
    let (actor, primitive, premise) = action_benefit_basis(
        action,
        continues_context,
        memory,
        turn,
        current_clarification,
    )
    .ok()?;
    let state = encode_support(&premise.atom.literal(premise.value));
    let selected = LiteralIR {
        proposition_id: "CONDITIONAL_ACTION_INTERPRETATION".into(),
        value: true,
    };
    let possible = LiteralIR {
        proposition_id: primitive.possible_effect_id.clone(),
        value: true,
    };
    let request = DeliberationRequestIR {
        schema: DELIBERATION_REQUEST_SCHEMA.into(),
        request_id: "ACTION_BENEFIT".into(),
        subject: premise.atom.id(),
        evidence: vec![
            EvidenceIR {
                evidence_id: "STATE".into(),
                literal: state.clone(),
                reliability_millis: 1000,
                source_ref: format!(
                    "USER_PREMISE:{}:{}",
                    premise.introduced_turn, premise.source_sha256
                ),
            },
            EvidenceIR {
                evidence_id: "INTERPRETATION".into(),
                literal: selected.clone(),
                reliability_millis: 1000,
                source_ref: "IF_THIS_ACTION_SENSE_IS_INTENDED_NOT_OBSERVATION".into(),
            },
        ],
        mechanisms: vec![CausalMechanismIR {
            mechanism_id: primitive.id.clone(),
            kind: MechanismKindIR::Inference,
            prerequisites: vec![state, selected],
            effects: vec![possible.clone()],
            observes: vec![],
            authority: ActionAuthorityIR::InternalInference,
            authorized: true,
            reversible: true,
            recovery_reference: None,
            cost_millis: 1,
            risk_millis: 0,
            provenance_refs: vec![primitive.source_ref.clone(), primitive.authorship.clone()],
        }],
        goals: vec![possible],
        authority_envelope: AuthorityEnvelopeIR {
            allow_internal_inference: true,
            allow_read_only_observation: false,
            allow_reversible_mutation: false,
            allow_irreversible_mutation: false,
            mutation_scope_id: None,
            ..Default::default()
        },
        immutable_constraints: vec![],
        max_depth: 16,
        beam_width: 16,
        max_hypotheses: 16,
        max_counterfactuals: 16,
    };
    let derivation = DeliberationEngine.deliberate(&request).ok()?;
    if derivation.disposition != DeliberationDispositionIR::GoalReachable {
        return None;
    }
    Some(ActionBenefitAssessmentIR {
        basis: Box::new(memory.clone()),
        evaluated_turn: turn,
        actor,
        primitive_id: primitive.id.clone(),
        request,
        derivation,
        current_clarification,
    })
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syntax_candidate: Option<Box<crate::world_vocabulary::WorldSyntaxCandidateIR>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldArgumentRoleIR {
    Subject,
    Object,
}

/// An open argument is not an entity or a world fact. Only a source-bound
/// reference reply can complete this partial predicate into a query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldArgumentGapIR {
    pub property: WorldPropertyIR,
    pub subject: Option<String>,
    pub object: Option<String>,
    pub missing_role: WorldArgumentRoleIR,
    pub positive: bool,
    pub explain: bool,
}

const OPEN_ARGUMENT_PROBE: &str = "__open_argument_probe__";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldReferenceGapIR {
    pub source_text: String,
    pub source_sha256: String,
    pub turn: u64,
    pub lexicon_revision: usize,
    pub context: WorldDiscourseIR,
    pub prior_query: Option<WorldQueryIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub argument: Option<WorldArgumentGapIR>,
}
impl WorldReferenceGapIR {
    pub fn candidates(&self) -> Vec<String> {
        self.context.referents()
    }

    fn infer_open_argument(&self, vocabulary: &WorldVocabularyIR) -> Option<WorldArgumentGapIR> {
        if !self.candidates().is_empty() || self.source_text.contains(OPEN_ARGUMENT_PROBE) {
            return None;
        }
        let ParsedInput::Query(query) = parse_input_with_vocabulary(
            &self.source_text,
            self.prior_query.as_ref(),
            vocabulary,
            self.lexicon_revision,
            &self.context,
            Some(OPEN_ARGUMENT_PROBE),
        )?
        else {
            return None;
        };
        if query.assumption.is_some() {
            return None;
        }
        let atom = query.target.0;
        let subject_missing = atom.entity == OPEN_ARGUMENT_PROBE;
        let object_missing = atom.object.as_deref() == Some(OPEN_ARGUMENT_PROBE);
        if subject_missing == object_missing {
            return None;
        }
        Some(WorldArgumentGapIR {
            property: atom.property,
            subject: (!subject_missing).then_some(atom.entity),
            object: if object_missing { None } else { atom.object },
            missing_role: if subject_missing {
                WorldArgumentRoleIR::Subject
            } else {
                WorldArgumentRoleIR::Object
            },
            positive: query.target.1,
            explain: query.explain,
        })
    }
    fn validate(&self, vocabulary: &WorldVocabularyIR) -> bool {
        if self.turn == 0
            || self.source_sha256 != hash(&self.source_text)
            || !self.context.validate(self.turn - 1, vocabulary)
        {
            return false;
        }
        let candidates = self.candidates();
        if parse_input_with_vocabulary(
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
        if candidates.is_empty() {
            return self.argument.is_some()
                && self.argument == self.infer_open_argument(vocabulary);
        }
        if candidates.len() != 2 || self.argument.is_some() {
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
    if gap.argument.is_some() {
        // Bind the requested identity, never the truth of its predicate.
        if boolean_reply(text).is_some() {
            return None;
        }
        if matches!(lower.as_str(), "me" | "i" | "나" | "저" | "나야" | "저요") {
            return Some("__user__".into());
        }
        let named = lower
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .or_else(|| lower.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
            .or_else(|| {
                ["이에요", "예요", "이야", "야"]
                    .iter()
                    .find_map(|ending| lower.strip_suffix(ending))
            });
        if matches!(named, Some("나" | "저")) {
            return Some("__user__".into());
        }
        return named
            .filter(|name| valid_entity(name) && *name != OPEN_ARGUMENT_PROBE)
            .map(str::to_string)
            .or_else(|| bare_reference_name(&lower).then(|| lower.clone()));
    }
    gap.candidates().into_iter().find(|c| {
        lower == *c
            || lower == format!("{c}야")
            || lower == format!("{c}이야")
            || (c == "__user__" && matches!(lower.as_str(), "me" | "나" | "저" | "나야"))
    })
}

fn bare_reference_name(text: &str) -> bool {
    if !valid_entity(text)
        || text == OPEN_ARGUMENT_PROBE
        || crate::conversation::is_discourse_only_fragment(text)
        || crate::pragmatics::detect_goal_withdrawal(text).is_some()
    {
        return false;
    }
    let lexical = crate::lexical_knowledge_pack::builtin_pack().lookup(text);
    // A requested identity slot supplies a nominal interpretation for an
    // otherwise unknown identifier. Known non-nominal/ambiguous forms require
    // explicit naming instead; POS evidence is not an entity truth claim.
    if lexical.truncated
        || (lexical.matches.is_empty()
            && crate::lexical_knowledge_pack::has_productive_finite_shape(text))
        || lexical
            .matches
            .iter()
            .any(|m| !matches!(m.entry.pos.as_str(), "명사" | "의존 명사" | "수사"))
    {
        return false;
    }
    // Korean action nouns can also be elliptical directives. A known
    // noun/하다-verb derivation is ambiguous here; require explicit naming.
    if !lexical.matches.is_empty() && text.chars().all(|c| ('가'..='힣').contains(&c)) {
        let derived = crate::lexical_knowledge_pack::builtin_pack().lookup(&format!("{text}하다"));
        if derived.truncated
            || derived
                .matches
                .iter()
                .any(|m| matches!(m.entry.pos.as_str(), "동사" | "보조 동사"))
        {
            return false;
        }
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldClarificationIR {
    pub memory: DialogueWorldIR,
    pub gap: WorldReferenceGapIR,
    /// A response to the pending dialogue act, not a new world proposition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub followup: Option<WorldClarificationFollowupIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldClarificationFollowupIR {
    pub source_text: String,
    pub turn: u64,
    pub kind: WorldClarificationFollowupKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<WorldClarificationActIR>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldClarificationFollowupKindIR {
    ReasonRequest,
    UnknownAnswer,
    DeclinedAnswer,
    Restatement,
}

/// Compact meaning of an emitted dialogue act; no generated sentence or world
/// snapshot. Repeated explanations retain a bounded, non-recursive basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldClarificationActIR {
    pub gap_identity: String,
    pub turn: u64,
    pub kind: WorldClarificationActKindIR,
    pub abstention: Option<WorldClarificationAbstentionIR>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorldClarificationActKindIR {
    RequestIdentity,
    ExplainRequirement,
    AllowAbstention,
    ExplainChoice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldClarificationAbstentionIR {
    pub source_text: String,
    pub turn: u64,
    pub kind: WorldClarificationFollowupKindIR,
}

impl WorldClarificationActIR {
    pub fn validate_for_gap(&self, gap: &WorldReferenceGapIR, completed_turns: u64) -> bool {
        use WorldClarificationActKindIR::*;
        self.gap_identity == hash(gap)
            && self.turn >= gap.turn
            && self.turn <= completed_turns
            && match self.kind {
                RequestIdentity | ExplainRequirement => self.abstention.is_none(),
                AllowAbstention | ExplainChoice => self.abstention.as_ref().is_some_and(|a| {
                    a.turn > gap.turn
                        && a.turn <= self.turn
                        && matches!(
                            a.kind,
                            WorldClarificationFollowupKindIR::UnknownAnswer
                                | WorldClarificationFollowupKindIR::DeclinedAnswer
                        )
                        && clarification_followup(&a.source_text) == Some(a.kind)
                }),
            }
    }

    pub fn validate_source(&self, gap: &WorldReferenceGapIR, source: &str, turn: u64) -> bool {
        use WorldClarificationActKindIR::*;
        use WorldClarificationFollowupKindIR::*;
        let operation = clarification_followup(source);
        self.turn == turn
            && self.validate_for_gap(gap, turn)
            && match self.kind {
                RequestIdentity => {
                    (turn == gap.turn && source == gap.source_text)
                        || operation == Some(Restatement)
                }
                ExplainRequirement => matches!(operation, Some(ReasonRequest | Restatement)),
                AllowAbstention => {
                    operation == Some(Restatement)
                        || self.abstention.as_ref().is_some_and(|a| {
                            a.source_text == source && a.turn == turn && operation == Some(a.kind)
                        })
                }
                ExplainChoice => {
                    matches!(operation, Some(ReasonRequest | Restatement))
                        && self.abstention.as_ref().is_some_and(|a| a.turn < turn)
                }
            }
    }
}

pub(crate) fn clarification_followup(text: &str) -> Option<WorldClarificationFollowupKindIR> {
    use WorldClarificationFollowupKindIR::*;
    // An unfilled explanation target is not by itself a complete request.
    // Use the interrogative grammar here; generic target detection below only
    // chooses which already-recognized act a complete request refers to.
    if clarification_reason_request(text) {
        return Some(ReasonRequest);
    }
    if crate::discourse_qa::is_answer_reformulation(text) {
        return Some(Restatement);
    }
    let question = text.trim().trim_end_matches(['?', '.', '!']).to_lowercase();
    let words = question.split_whitespace().collect::<Vec<_>>();
    if matches!(words.as_slice(), ["what", "do", "you", "mean"])
        || matches!(words.as_slice(), ["무슨", predicate] if predicate.strip_prefix("뜻").is_some_and(|ending| matches!(ending, "이야" | "이에요" | "인가요" | "입니까")))
    {
        return Some(Restatement);
    }
    let normalized = text
        .trim()
        .trim_end_matches(['.', '!'])
        .to_lowercase()
        .replace('’', "'");
    if normalized.chars().count() > 256
        || normalized.contains(['?', '"', '“', '”'])
        || normalized.starts_with('\'')
        || normalized.ends_with('\'')
    {
        return None;
    }
    let tokens: Vec<_> = normalized.split_whitespace().take(9).collect();
    if tokens.is_empty() || tokens.len() > 8 {
        return None;
    }
    let (predicate, prefix) = tokens.split_last()?;
    let current_finite = crate::lexical_knowledge_pack::is_connective_principal_form(
        predicate.strip_suffix('요').unwrap_or(predicate),
    ) || [
        "어",
        "어요",
        "아",
        "아요",
        "네",
        "네요",
        "지",
        "죠",
        "습니다",
        "거든",
        "거든요",
        "잖아",
        "잖아요",
    ]
    .iter()
    .any(|ending| predicate.ends_with(ending));
    if current_finite {
        let lexical = crate::lexical_knowledge_pack::builtin_pack().lookup(predicate);
        let present_lemma = |lemma: &str| {
            lexical
                .matches
                .iter()
                .any(|m| m.entry.lemma == lemma && !m.morphology.grammar_rule.contains("PAST"))
        };
        if !lexical.truncated
            && present_lemma("모르다")
            && prefix.iter().all(|word| {
                matches!(
                    *word,
                    "나" | "나는"
                        | "나도"
                        | "저"
                        | "저는"
                        | "저도"
                        | "잘"
                        | "아직"
                        | "정확히"
                        | "정말"
                        | "그건"
                        | "그걸"
                        | "답은"
                        | "답을"
                )
            })
        {
            return Some(UnknownAnswer);
        }
        if !lexical.truncated && present_lemma("싫다") {
            let rest = if prefix
                .first()
                .is_some_and(|w| matches!(*w, "나" | "나는" | "저" | "저는"))
            {
                &prefix[1..]
            } else {
                prefix
            };
            let communication = rest.len() == 1
                && rest[0].strip_suffix('기').is_some_and(|stem| {
                    let word =
                        crate::lexical_knowledge_pack::builtin_pack().lookup(&format!("{stem}다"));
                    !word.truncated
                        && word.matches.iter().any(|m| {
                            matches!(
                                m.entry.lemma.as_str(),
                                "말하다" | "답하다" | "대답하다" | "알려주다"
                            )
                        })
                });
            if rest.is_empty() || communication {
                return Some(DeclinedAnswer);
            }
        }
        if !lexical.truncated && present_lemma("않다") {
            let rest = if prefix
                .first()
                .is_some_and(|w| matches!(*w, "나는" | "저는" | "나" | "저"))
            {
                &prefix[1..]
            } else {
                prefix
            };
            if let [communication, "싶지"] = rest {
                if communication.strip_suffix('고').is_some_and(|stem| {
                    let lexical =
                        crate::lexical_knowledge_pack::builtin_pack().lookup(&format!("{stem}다"));
                    !lexical.truncated
                        && lexical.matches.iter().any(|m| {
                            matches!(
                                m.entry.lemma.as_str(),
                                "말하다" | "답하다" | "대답하다" | "알려주다"
                            )
                        })
                }) {
                    return Some(DeclinedAnswer);
                }
            }
        }
    }
    // Expand function-word contractions, then compose speaker, negation,
    // epistemic/volitional predicate and its (absent) content complement.
    let expanded: Vec<_> = tokens
        .iter()
        .flat_map(|word| match *word {
            "don't" => vec!["do", "not"],
            "i'm" => vec!["i", "am"],
            "i'd" => vec!["i", "would"],
            "won't" => vec!["will", "not"],
            other => vec![other],
        })
        .filter(|w| !matches!(*w, "really" | "quite" | "yet"))
        .collect();
    let speaker = expanded.first() == Some(&"i");
    let rest = if speaker {
        &expanded[1..]
    } else {
        &expanded[..]
    };
    if matches!(rest, ["do", "not", "know"] | ["not", "sure"])
        || (speaker && rest == ["am", "not", "sure"])
    {
        return Some(UnknownAnswer);
    }
    if speaker {
        let communication = match rest {
            ["would", "rather", "not", verb]
            | ["will", "not", verb]
            | ["do", "not", "want", "to", verb] => Some(*verb),
            _ => None,
        };
        if communication.is_some_and(|v| matches!(v, "say" | "answer" | "tell")) {
            return Some(DeclinedAnswer);
        }
    }
    None
}

pub(crate) fn clarification_reason_request(text: &str) -> bool {
    let normalized = text.trim().trim_end_matches(['?', '.', '!']).to_lowercase();
    let tokens: Vec<_> = normalized.split_whitespace().collect();
    // Bounded interrogative grammar: causal WH + optional interlocutor and
    // asking predicate. Explicit world topics and commands do not match.
    let Some((first, rest)) = tokens.split_first() else {
        return false;
    };
    match *first {
        "why" => {
            rest.is_empty()
                || matches!(
                    rest,
                    ["ask"]
                        | ["do", "you", "ask"]
                        | ["did", "you", "ask"]
                        | ["are", "you", "asking"]
                )
        }
        "왜" => {
            if rest.is_empty() {
                return true;
            }
            let rest = rest.strip_prefix(&["그걸"]).unwrap_or(rest);
            matches!(
                rest,
                ["물어"] | ["물어요"] | ["묻는", "거야"] | ["묻는", "거예요"]
            ) || matches!(rest, [predicate] if {
                let lexical = crate::lexical_knowledge_pack::builtin_pack().lookup(predicate);
                ["어", "어요", "니", "나요", "습니까", "나"].iter().any(|ending| predicate.ends_with(ending))
                    && !lexical.truncated && lexical.matches.iter().any(|m| matches!(m.entry.lemma.as_str(), "묻다" | "물어보다" | "질문하다") && m.morphology.grammar_rule.contains("PAST"))
            })
        }
        _ => false,
    }
}

impl WorldClarificationIR {
    pub fn response_act(&self) -> WorldClarificationActIR {
        use WorldClarificationActKindIR::*;
        use WorldClarificationFollowupKindIR::*;
        let mut act = WorldClarificationActIR {
            gap_identity: hash(&self.gap),
            turn: self.response_turn(),
            kind: RequestIdentity,
            abstention: None,
        };
        if let Some(r) = &self.followup {
            match r.kind {
                UnknownAnswer | DeclinedAnswer => {
                    act.kind = AllowAbstention;
                    act.abstention = Some(WorldClarificationAbstentionIR {
                        source_text: r.source_text.clone(),
                        turn: r.turn,
                        kind: r.kind,
                    });
                }
                ReasonRequest => {
                    act.kind = ExplainRequirement;
                    if let Some(anchor) = &r.anchor {
                        if matches!(anchor.kind, AllowAbstention | ExplainChoice) {
                            act.kind = ExplainChoice;
                            act.abstention = anchor.abstention.clone();
                        }
                    }
                }
                Restatement => {
                    if let Some(anchor) = &r.anchor {
                        act.kind = anchor.kind;
                        act.abstention = anchor.abstention.clone();
                    }
                }
            }
        }
        act
    }
    pub fn response_source(&self) -> &str {
        self.followup
            .as_ref()
            .map_or(&self.gap.source_text, |r| &r.source_text)
    }
    pub fn response_turn(&self) -> u64 {
        self.followup.as_ref().map_or(self.gap.turn, |r| r.turn)
    }
    pub fn validate(&self) -> bool {
        self.memory.validate(self.memory.latest_turn())
            && self.gap.validate(&self.memory.vocabulary)
            && self.memory.pending_reference.as_ref() == Some(&self.gap)
            && self.followup.as_ref().is_none_or(|r| {
                r.turn > self.gap.turn
                    && clarification_followup(&r.source_text) == Some(r.kind)
                    && (r.kind != WorldClarificationFollowupKindIR::Restatement
                        || r.anchor.is_some())
                    && r.anchor.as_ref().is_none_or(|a| {
                        matches!(
                            r.kind,
                            WorldClarificationFollowupKindIR::ReasonRequest
                                | WorldClarificationFollowupKindIR::Restatement
                        ) && a.validate_for_gap(&self.gap, r.turn.saturating_sub(1))
                            && r.turn.saturating_sub(a.turn) <= 4
                            && (r.kind != WorldClarificationFollowupKindIR::ReasonRequest
                                || crate::discourse_qa::unbound_explanation_target(&r.source_text)
                                    .is_some())
                    })
            })
    }
    pub fn into_answer(
        self,
        language: crate::language_knowledge::LanguageCodeIR,
    ) -> Result<crate::discourse_qa::DiscourseAnswerIR, String> {
        let mut answer =
            crate::discourse_qa::DiscourseQaEngine.unanswered(self.response_source(), language);
        answer.claims.clear();
        answer.disposition = crate::discourse_qa::DiscourseAnswerDispositionIR::AmbiguousQuery;
        answer.world_clarification = Some(self);
        answer.refresh_structured_preview();
        answer
            .validate()
            .then_some(answer)
            .ok_or_else(|| "INVALID_REFERENCE_GAP".into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldQueryIR {
    /// A distinct pending semantic goal may ask for this unknown premise.
    /// Ordinary user factual queries leave this absent (no echo-question).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clarification_goal: Option<String>,
    pub target: (WorldAtomIR, bool),
    pub explain: bool,
    /// A local intervention is never written into factual memory.
    pub assumption: Option<(WorldAtomIR, bool)>,
}

impl WorldQueryIR {
    fn valid_clarification_scope(&self) -> bool {
        self.clarification_goal.as_ref().is_none_or(|goal| {
            !self.explain
                && self.assumption.is_none()
                && self.target.1
                && goal
                    .strip_prefix("ACTION_BENEFIT:")
                    .is_some_and(|id| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()))
        })
    }
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
    Revision([(WorldAtomIR, bool); 2]),
    Implication(Vec<(WorldAtomIR, bool)>, (WorldAtomIR, bool)),
    Query(WorldQueryIR),
}
impl ParsedInput {
    fn asserts(&self, atom: &WorldAtomIR, value: bool) -> bool {
        match self {
            Self::Premise((a, v), _) => a == atom && *v == value,
            Self::Revision(pairs) => pairs.iter().any(|(a, v)| a == atom && *v == value),
            _ => false,
        }
    }
    fn corrects(&self, atom: &WorldAtomIR) -> bool {
        match self {
            Self::Premise((a, _), true) => a == atom,
            Self::Revision(pairs) => pairs.iter().any(|(a, _)| a == atom),
            _ => false,
        }
    }
}

pub struct PreparedWorldTurn {
    pub memory: DialogueWorldIR,
    pub query: Option<WorldQueryIR>,
    pub recognized: bool,
    pub clarification: Option<WorldClarificationIR>,
    /// Structural syntax evidence is exposed for candidate-only evaluation;
    /// it never grants semantic authority by itself.
    pub syntax_candidate: Option<Box<crate::world_vocabulary::WorldSyntaxCandidateIR>>,
}

/// Acknowledgement of a typed memory update, not an inferred world fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMemoryUpdateIR {
    pub memory: DialogueWorldIR,
    pub turn: u64,
    pub source_text: String,
}
impl WorldMemoryUpdateIR {
    pub(crate) fn is_bound_boolean_answer(&self) -> bool {
        boolean_reply(&self.source_text).is_some_and(|value| {
            self.memory.premises.last().is_some_and(|p| {
                p.introduced_turn == self.turn
                    && p.active
                    && p.source_text == self.source_text
                    && p.value == value
                    && p.answer_binding.is_some()
            })
        })
    }

    pub fn validate(&self) -> bool {
        self.memory.validate(self.turn)
            && (self.is_bound_boolean_answer()
                || match self.memory.parse_at(
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
                    Some(ParsedInput::Revision(pairs)) => pairs.iter().all(|(atom, value)| {
                        self.memory.premises.iter().any(|p| {
                            p.introduced_turn == self.turn
                                && p.active
                                && &p.atom == atom
                                && &p.value == value
                                && p.source_text == self.source_text
                        })
                    }),
                    Some(ParsedInput::Implication(prerequisites, effect)) => {
                        self.memory.implications.last().is_some_and(|r| {
                            r.introduced_turn == self.turn
                                && r.prerequisites == prerequisites
                                && r.effect == effect
                                && r.source_text == self.source_text
                        })
                    }
                    _ => false,
                })
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
        answer.world_memory_update = Some(self);
        answer.refresh_structured_preview();
        answer
            .validate()
            .then_some(answer)
            .ok_or_else(|| "INVALID_WORLD_MEMORY_UPDATE".into())
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
        self.prepare_with_act(text, turn, None)
    }

    pub(crate) fn prepare_with_act(
        &self,
        text: &str,
        turn: u64,
        prior_act: Option<&WorldClarificationActIR>,
    ) -> Result<PreparedWorldTurn, String> {
        if !self.validate(turn.saturating_sub(1)) {
            return Err("INVALID_WORLD_MEMORY".into());
        }
        let mut memory = self.clone();
        // Do not run lexical followup analysis on turns without a pending act.
        if let Some(gap) = self.pending_reference.as_ref() {
            if let Some(kind) = clarification_followup(text) {
                let anchor = prior_act
                    .filter(|a| {
                        a.validate_for_gap(gap, turn.saturating_sub(1))
                            && turn.saturating_sub(a.turn) <= 4
                            && (kind == WorldClarificationFollowupKindIR::Restatement
                                || (kind == WorldClarificationFollowupKindIR::ReasonRequest
                                    && crate::discourse_qa::unbound_explanation_target(text)
                                        .is_some()))
                    })
                    .cloned();
                if kind != WorldClarificationFollowupKindIR::Restatement || anchor.is_some() {
                    return Ok(PreparedWorldTurn {
                        clarification: Some(WorldClarificationIR {
                            memory: memory.clone(),
                            gap: gap.clone(),
                            followup: Some(WorldClarificationFollowupIR {
                                source_text: text.into(),
                                turn,
                                kind,
                                anchor,
                            }),
                        }),
                        memory,
                        query: None,
                        recognized: true,
                        syntax_candidate: None,
                    });
                }
            }
        }
        let resolution = self.pending_reference.as_ref().and_then(|gap| {
            selected_reference(text, gap).map(|selected| WorldReferenceResolutionIR {
                gap: gap.clone(),
                selected,
            })
        });
        let syntax_source_sha256 = hash(&text);
        let syntax_lexicon_revision = self.vocabulary.revision();
        let syntax_candidate =
            self.vocabulary
                .syntax_candidate(text, syntax_lexicon_revision, &syntax_source_sha256);
        let mut parsed = self.parse_at(
            text,
            self.last_query.as_ref(),
            syntax_lexicon_revision,
            &self.discourse,
            resolution.as_ref(),
        );
        if parsed.is_none() && resolution.is_none() {
            let mut gap = WorldReferenceGapIR {
                source_text: text.into(),
                source_sha256: hash(&text),
                turn,
                lexicon_revision: self.vocabulary.revision(),
                context: self.discourse.clone(),
                prior_query: self.last_query.clone(),
                argument: None,
            };
            gap.argument = gap.infer_open_argument(&self.vocabulary);
            if gap.validate(&self.vocabulary) {
                memory.pending_reference = Some(gap.clone());
                memory.last_grounding = None;
                return Ok(PreparedWorldTurn {
                    clarification: Some(WorldClarificationIR {
                        memory: memory.clone(),
                        gap,
                        followup: None,
                    }),
                    memory,
                    query: None,
                    recognized: true,
                    syntax_candidate: syntax_candidate.map(Box::new),
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
            syntax_candidate: syntax_candidate
                .as_ref()
                .map(|candidate| Box::new(candidate.clone())),
        });
        let mut query = None;
        match parsed {
            Some(ParsedInput::Revision(pairs)) => {
                if memory.premises.len() + pairs.len() > CAPACITY {
                    return Err("WORLD_PREMISE_CAPACITY".into());
                }
                for (atom, value) in pairs {
                    for prior in &mut memory.premises {
                        if prior.atom == atom {
                            prior.active = false;
                        }
                    }
                    memory.discourse = WorldDiscourseIR {
                        focus: Some((atom.clone(), value)),
                        source_turn: turn,
                    };
                    memory.premises.push(WorldPremiseIR {
                        atom,
                        value,
                        introduced_turn: turn,
                        source_text: text.into(),
                        source_sha256: hash(&text),
                        active: true,
                        answer_binding: None,
                        lexicon_revision: self.vocabulary.revision(),
                        grounding_context: self.discourse.clone(),
                        reference_resolution: resolution.clone(),
                    });
                }
                memory.last_query = None;
            }
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
                if !crate::utterance_intent::decision_inquiry(text)
                    .is_some_and(|inquiry| inquiry.continues_context)
                    && !matches!(
                        text.trim()
                            .trim_end_matches(['.', '!'])
                            .to_lowercase()
                            .as_str(),
                        "thanks" | "고마워" | "감사합니다"
                    )
                {
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
            syntax_candidate: syntax_candidate.map(Box::new),
        })
    }

    pub fn validate(&self, turn: u64) -> bool {
        self.vocabulary.validate() && self.premises.len() <= CAPACITY && self.implications.len() <= CAPACITY
            && self.premises.windows(2).all(|pair| pair[0].introduced_turn < pair[1].introduced_turn
                || (pair[0].introduced_turn == pair[1].introduced_turn
                    && pair[0].source_text == pair[1].source_text
                    && matches!(self.parse_at(&pair[0].source_text, None, pair[0].lexicon_revision,
                        &pair[0].grounding_context, pair[0].reference_resolution.as_ref()), Some(ParsedInput::Revision(_)))))
            && self.premises.iter().map(|p| (p.introduced_turn, p.atom.id())).collect::<BTreeSet<_>>().len() == self.premises.len()
            && self.implications.windows(2).all(|pair| pair[0].introduced_turn < pair[1].introduced_turn)
            && self.premises.iter().all(|p| p.active != self.premises.iter().any(|later|
                later.introduced_turn > p.introduced_turn && later.atom == p.atom
                && self.parse_at(&later.source_text, None, later.lexicon_revision, &later.grounding_context, later.reference_resolution.as_ref()).is_some_and(|parsed| parsed.corrects(&p.atom))))
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
                } else {self.parse_at(&p.source_text, None, p.lexicon_revision, &p.grounding_context, p.reference_resolution.as_ref()).is_some_and(|parsed| parsed.asserts(&p.atom, p.value))})
            && self.implications.iter().all(|r| r.introduced_turn > 0 && r.introduced_turn <= turn
                && r.source_sha256 == hash(&r.source_text)
                && r.grounding_context.validate(r.introduced_turn.saturating_sub(1), &self.vocabulary)
                && matches!(self.parse_at(&r.source_text, None, r.lexicon_revision, &r.grounding_context, r.reference_resolution.as_ref()), Some(ParsedInput::Implication(p,e)) if p == r.prerequisites && e == r.effect))
            && self.last_query.as_ref().is_none_or(|q| q.valid_clarification_scope() && valid_atom(&q.target.0, &self.vocabulary)
                && q.assumption.as_ref().is_none_or(|(a,_)| valid_atom(a, &self.vocabulary)))
            && self.answer_bindings_valid()
            && self.revision_episodes_complete()
            && self.discourse.validate(turn, &self.vocabulary)
            && self.pending_reference.as_ref().is_none_or(|g| g.turn<=turn && g.validate(&self.vocabulary))
            && self.last_grounding.as_ref().is_none_or(|g| g.turn > 0 && g.turn <= turn
                && g.source_sha256 == hash(&g.source_text)
                && g.context.validate(g.turn-1,&self.vocabulary)
                && g.lexicon_revision <= self.vocabulary.revision()
                && self.grounding_matches_memory(g))
    }

    fn revision_episodes_complete(&self) -> bool {
        self.premises.iter().all(|p| {
            let Some(ParsedInput::Revision(pairs)) = self.parse_at(
                &p.source_text,
                None,
                p.lexicon_revision,
                &p.grounding_context,
                p.reference_resolution.as_ref(),
            ) else {
                return true;
            };
            pairs.iter().all(|(atom, value)| {
                self.premises.iter().any(|other| {
                    other.introduced_turn == p.introduced_turn
                        && other.source_text == p.source_text
                        && other.lexicon_revision == p.lexicon_revision
                        && other.grounding_context == p.grounding_context
                        && other.reference_resolution == p.reference_resolution
                        && other.answer_binding.is_none()
                        && &other.atom == atom
                        && &other.value == value
                })
            })
        })
    }

    fn grounding_matches_memory(&self, g: &WorldInputGroundingIR) -> bool {
        if g.source_sha256 != hash(&g.source_text)
            || g.syntax_candidate.as_deref()
                != self
                    .vocabulary
                    .syntax_candidate(&g.source_text, g.lexicon_revision, &g.source_sha256)
                    .as_ref()
        {
            return false;
        }
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
            Some(ParsedInput::Revision(pairs)) => pairs.iter().all(|(a, v)| {
                self.premises.iter().any(|p| {
                    p.introduced_turn == g.turn
                        && p.source_text == g.source_text
                        && &p.atom == a
                        && &p.value == v
                        && p.grounding_context == g.context
                })
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
                        self.parse_at(
                            &earlier.source_text,
                            None,
                            earlier.lexicon_revision,
                            &earlier.grounding_context,
                            earlier.reference_resolution.as_ref(),
                        )
                        .is_some_and(|parsed| parsed.corrects(&earlier.atom))
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
    crate::conversation::confirmation_polarity(text)
}

fn valid_atom(atom: &WorldAtomIR, vocabulary: &WorldVocabularyIR) -> bool {
    vocabulary.accepts_atom(atom)
        && valid_entity(&atom.entity)
        && atom.object.as_ref().is_none_or(|o| valid_entity(o))
}

fn valid_entity(entity: &str) -> bool {
    !entity.is_empty()
        && !crate::conversation_contract::is_interrogative_placeholder(entity)
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
    attributive: bool,
    allow_question_forms: bool,
) -> Option<(WorldAtomIR, bool)> {
    let lower = text
        .trim()
        .trim_end_matches(['.', '?', '!'])
        .trim()
        .to_lowercase();
    vocabulary.lexical_history.get(revision)?;
    if let Some((atom, value)) =
        vocabulary.parse_form(&lower, revision, attributive, allow_question_forms)
    {
        return Some((atom, value));
    }
    let (entity, predicate) = copular_parts(&lower)?;
    let (predicate, value) = if attributive {
        crate::world_vocabulary::copular_attributive_root(predicate)?
    } else {
        copular_root(predicate)
    };
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
    speaker_default: bool,
    attributive: bool,
    allow_question_forms: bool,
) -> Option<(WorldAtomIR, bool)> {
    if !speaker_default
        && crate::proposition_content::described_event(text, true).is_some_and(|q| {
            q.kind == crate::proposition_content::DescriptionKindIR::State
                && !q
                    .roles
                    .contains_key(&crate::proposition_content::ContentSlotIR::Theme)
        })
    {
        // A typed open bearer is not a Boolean query over a named world atom.
        // Consume this distinction before Korean case morphology removes the
        // source WH form. The discourse query path owns variable binding.
        return None;
    }
    let parse = |s: &str| {
        parse_atom_with_vocabulary(s, vocabulary, revision, attributive, allow_question_forms)
    };
    let (mut atom, value) = parse(text)
        .or_else(|| reference.and_then(|_| parse(&format!("it is {text}"))))
        .or_else(|| reference.and_then(|_| parse(&format!("그것은 {text}"))))
        .or_else(|| {
            // Ellipsis is a discourse operation. Never bypass a prior referent
            // or infer the speaker for an unbound question/general predicate.
            (speaker_default && reference.is_none())
                .then(|| parse(&format!("__user__는 {text}")))
                .flatten()
                .filter(|(atom, _)| {
                    vocabulary.lexical_history.get(revision).is_some_and(|aliases| {
                        aliases.values().any(|alias| {
                            atom.property == WorldPropertyIR::Registered(alias.predicate_id.clone())
                                && alias.grammar == crate::world_vocabulary::WorldLexicalGrammarIR::KoreanHadaExperiencer
                        })
                    })
                })
        })?;
    for entity in std::iter::once(&mut atom.entity).chain(atom.object.iter_mut()) {
        if matches!(entity.as_str(), "i" | "me" | "나" | "저" | "내" | "제") {
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
    let registered_hada_question = vocabulary.has_unary_hada_question_form(text, revision);
    let speaker_default =
        !crate::conversation_contract::is_interrogative(lower) && !registered_hada_question;
    let parse_atom = |text: &str| {
        parse_contextual_atom(
            text,
            vocabulary,
            revision,
            reference,
            speaker_default,
            false,
            false,
        )
    };
    let parse_question_atom = |text: &str| {
        parse_contextual_atom(text, vocabulary, revision, reference, false, false, true)
    };
    if let Some(question) = crate::proposition_content::nominal_relation_question(text) {
        if question.slot != crate::proposition_content::ContentSlotIR::Cause {
            return None;
        }
        let target = parse_contextual_atom(
            question.complement,
            vocabulary,
            revision,
            reference,
            false,
            question.korean_attributive,
            false,
        )?;
        return Some(ParsedInput::Query(WorldQueryIR {
            clarification_goal: None,
            target,
            explain: true,
            assumption: None,
        }));
    }
    // A target-free explanation is a discourse operation over the focused
    // proposition, not a new world topic. Reuse the same grammatical ownership
    // test as missing-target clarification; an explicit target never enters here.
    let why = matches!(text, "why" | "왜" | "왜 그래" | "그 이유는")
        || crate::discourse_qa::unbound_explanation_target(text)
            == Some(crate::discourse_qa::DiscourseQueryKindIR::MissingExplanationTarget);
    if why || matches!(text, "so" | "then" | "그럼" | "그러면") {
        return last
            .cloned()
            .or_else(|| {
                context.focus.clone().map(|target| WorldQueryIR {
                    clarification_goal: None,
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
                clarification_goal: None,
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
        let target = parse_question_atom(question)?;
        return Some(ParsedInput::Query(WorldQueryIR {
            clarification_goal: None,
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
    // Punctuationless Korean HADA questions need grammar-owned evidence: the
    // generic interrogative classifier cannot distinguish `한가` from a
    // descriptive surface without consulting the registered unary lexeme.
    // Keep this scoped to the vocabulary so binary relations and unknown roots
    // do not become world queries.
    if crate::conversation_contract::is_interrogative(lower) || why || registered_hada_question {
        return parse_question_atom(body).map(|target| {
            ParsedInput::Query(WorldQueryIR {
                clarification_goal: None,
                target,
                explain: why,
                assumption: None,
            })
        });
    }
    let (body, correction) = crate::proposition_content::strip_correction_prefix(text);
    if ["하면", "않으면", "하고", "않고"]
        .iter()
        .any(|ending| body.ends_with(ending))
    {
        return None;
    }
    let contrast = body
        .split_once(" 게 아니라 ")
        .or_else(|| body.split_once(" 것이 아니라 "));
    if let Some((rejected, replacement)) = contrast {
        let left = nominalized_hada(rejected)?;
        let (rejected_atom, rejected_value) = parse_atom(&left)?;
        let right = nominalized_hada(
            replacement
                .trim_end_matches(" 거야")
                .trim_end_matches(" 것이야"),
        )?;
        let bound = format!("{}는 {right}", rejected_atom.entity);
        let (replacement_atom, replacement_value) =
            parse_atom(&bound).or_else(|| parse_atom(&right))?;
        if rejected_atom.entity != replacement_atom.entity || rejected_atom == replacement_atom {
            return None;
        }
        return Some(ParsedInput::Revision([
            (rejected_atom, !rejected_value),
            (replacement_atom, replacement_value),
        ]));
    }
    if let Some((left, right)) = body
        .split_once(" but ")
        .filter(|(left, _)| left.contains("not "))
    {
        let rejected = parse_atom(left)?;
        let right = right.trim();
        let bound = format!("{} is {right}", rejected.0.entity);
        let replacement = parse_atom(&bound).or_else(|| parse_atom(right))?;
        if rejected.0.entity != replacement.0.entity || rejected.0 == replacement.0 {
            return None;
        }
        return Some(ParsedInput::Revision([rejected, replacement]));
    }
    parse_atom(body).map(|a| ParsedInput::Premise(a, correction))
}

fn nominalized_hada(text: &str) -> Option<String> {
    text.strip_suffix('한').map(|stem| format!("{stem}하다"))
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
    // This turn's communicative goal is acquiring a missing premise for a
    // different question, not reporting the premise's unknown truth value.
    if query.clarification_goal.is_some() && decision.verdict == WorldVerdictIR::Unknown {
        if let Some(question) = &decision.question {
            return Ok(WorldUtterancePlanIR {
                decision_sha256: hash(decision),
                explains: false,
                moves: vec![WorldUtteranceMoveIR {
                    proposition: question.clone(),
                    purpose: P::Ask,
                    evidence_refs: vec!["MISSING_PREMISE".into()],
                }],
            });
        }
    }
    // An explanation request does not establish its presupposition. If the
    // target is refuted, communicate the supported correction; do not answer
    // an unasked why-question about the opposite proposition.
    if query.explain
        && decision.verdict == WorldVerdictIR::Supported
        && decision.proof_mechanism_ids.is_empty()
        && !decision.hypothetical
    {
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
    if query.explain
        && decision.conclusion.is_some()
        && (decision.verdict != WorldVerdictIR::Refuted || !decision.proof_mechanism_ids.is_empty())
    {
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
        answer.world_reasoning = Some(self);
        answer.refresh_structured_preview();
        answer
            .validate()
            .then_some(answer)
            .ok_or_else(|| "INVALID_WORLD_DECISION".into())
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
    if !query.valid_clarification_scope() || !valid_atom(&query.target.0, &memory.vocabulary) {
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
            .filter(|id| **id != query.target.0.id() || query.clarification_goal.is_some())
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

    #[test]
    fn action_aliases_are_indexed_deduplicated_and_cannot_mutate_primitives() {
        let primitives: Vec<ActionBenefitPrimitiveIR> = serde_json::from_str(include_str!(
            "../data/lexical-knowledge/action-benefit-primitives.json"
        ))
        .unwrap();
        let language: ActionBenefitLanguageIR = serde_json::from_str(include_str!(
            "../data/lexical-knowledge/action-benefit-language.json"
        ))
        .unwrap();
        let action = crate::utterance_intent::decision_inquiry("Then would it be good to rest?")
            .unwrap()
            .proposed_action
            .unwrap();
        let original = ActionBenefitKnowledge::new(primitives.clone(), language.clone()).unwrap();
        let semantic_bytes = serde_json::to_vec(&original.primitives).unwrap();
        assert_eq!(
            original.candidates(&action),
            BTreeSet::from(["ABP_0001".into()])
        );

        let mut renamed = language.clone();
        renamed.expressions[0].action_ko = "다른 표제".into();
        renamed.expressions[0].action_en = "opaque label".into();
        let renamed = ActionBenefitKnowledge::new(primitives.clone(), renamed).unwrap();
        assert_eq!(
            semantic_bytes,
            serde_json::to_vec(&renamed.primitives).unwrap()
        );
        assert_eq!(original.candidates(&action), renamed.candidates(&action));

        let unnamed = ActionBenefitKnowledge::new(
            primitives.clone(),
            ActionBenefitLanguageIR {
                bindings: vec![],
                expressions: vec![],
            },
        )
        .unwrap();
        assert_eq!(
            semantic_bytes,
            serde_json::to_vec(&unnamed.primitives).unwrap()
        );
        assert!(unnamed.candidates(&action).is_empty());

        // Adding a second lexical route to one primitive is not a competing
        // hypothesis. Irrelevant meanings are not examined during selection.
        let mut augmented = primitives.clone();
        let mut indexed_language = language.clone();
        for i in 1..32 {
            let mut p = primitives[0].clone();
            p.id = format!("DISTRACTOR_{i}");
            indexed_language.bindings.push(ActionBenefitBindingIR {
                primitive_id: p.id.clone(),
                entry_id: "68752".into(),
                sense_id: "1".into(),
                source_ref: "TEST_ONLY_IRRELEVANT_BINDING".into(),
            });
            augmented.push(p);
        }
        let augmented = ActionBenefitKnowledge::new(augmented, indexed_language).unwrap();
        assert_eq!(augmented.primitives.len(), 32);
        assert_eq!(augmented.candidates(&action), original.candidates(&action));

        let mut dangling = language.clone();
        dangling.bindings[0].primitive_id = "ABSENT".into();
        assert!(ActionBenefitKnowledge::new(primitives.clone(), dangling).is_err());
        let mut unknown_sense = language;
        unknown_sense.bindings[0].sense_id = "UNKNOWN_SENSE".into();
        assert!(ActionBenefitKnowledge::new(primitives, unknown_sense).is_err());
    }
    use crate::world_vocabulary::{
        WorldLexemeIR, WorldLexicalGrammarIR as G, WorldPredicateArityIR as A,
        WorldPredicateSpecIR, WorldSyntaxModelIR, WorldSyntaxObservationIR,
        WorldSyntaxResolutionModeIR, WorldVocabularyUpdateIR,
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

    fn learned_object_subject_model(mode: WorldSyntaxResolutionModeIR) -> WorldSyntaxModelIR {
        WorldSyntaxModelIR::train_with_mode(
            "test-role-order",
            "v1",
            mode,
            &[
                WorldSyntaxObservationIR {
                    language: LanguageCodeIR::Korean,
                    grammar: G::KoreanHadaLocative,
                    subject_particle: "가".into(),
                    object_particle: "에".into(),
                    subject_position: 1,
                    object_position: 0,
                    source_ref: "structural-observation-a".into(),
                    source_version: "fixture-v1".into(),
                },
                WorldSyntaxObservationIR {
                    language: LanguageCodeIR::Korean,
                    grammar: G::KoreanHadaLocative,
                    subject_particle: "는".into(),
                    object_particle: "에".into(),
                    subject_position: 1,
                    object_position: 0,
                    source_ref: "structural-observation-b".into(),
                    source_version: "fixture-v1".into(),
                },
            ],
        )
        .unwrap()
    }

    #[test]
    fn learned_binary_role_order_uses_existing_memory_and_proof_path() {
        let vocabulary = WorldVocabularyIR::default()
            .updated(&vocabulary_update())
            .unwrap()
            .with_syntax_model(learned_object_subject_model(
                WorldSyntaxResolutionModeIR::Resolve,
            ))
            .unwrap();
        let world = DialogueWorldIR {
            vocabulary,
            ..Default::default()
        };
        let prepared = world.prepare("릴레이에 시더가 의존한다", 1).unwrap();
        assert!(prepared.recognized);
        assert_eq!(prepared.memory.premises.len(), 1);
        assert_eq!(
            prepared.memory.premises[0].atom,
            WorldAtomIR {
                entity: "시더".into(),
                property: WorldPropertyIR::Registered("W_USER_2".into()),
                object: Some("릴레이".into()),
            }
        );
        let candidate = prepared.syntax_candidate.as_ref().unwrap();
        assert_eq!(
            candidate.order,
            crate::world_vocabulary::WorldBinaryOrderIR::ObjectSubject
        );
        assert!(!candidate.semantic_authority);
        assert_eq!(candidate.source_sha256, hash(&"릴레이에 시더가 의존한다"));
        let reasoning = reason(&prepared.memory, "시더는 릴레이에 의존하나요?");
        assert_eq!(reasoning.decision.verdict, WorldVerdictIR::Supported);
    }

    #[test]
    fn candidate_only_role_model_never_commits_a_fact() {
        let vocabulary = WorldVocabularyIR::default()
            .updated(&vocabulary_update())
            .unwrap()
            .with_syntax_model(learned_object_subject_model(
                WorldSyntaxResolutionModeIR::CandidateOnly,
            ))
            .unwrap();
        let world = DialogueWorldIR {
            vocabulary,
            ..Default::default()
        };
        let prepared = world.prepare("릴레이에 시더가 의존한다", 1).unwrap();
        assert!(!prepared.recognized);
        assert!(prepared.syntax_candidate.is_some());
        assert!(prepared.memory.premises.is_empty());
        assert!(prepared.memory.last_grounding.is_none());
    }

    #[test]
    fn canonical_argument_order_wins_over_learned_candidate_fallback() {
        let vocabulary = WorldVocabularyIR::default()
            .updated(&vocabulary_update())
            .unwrap()
            .with_syntax_model(learned_object_subject_model(
                WorldSyntaxResolutionModeIR::Resolve,
            ))
            .unwrap();
        let world = DialogueWorldIR {
            vocabulary,
            ..Default::default()
        };
        let prepared = world.prepare("시더는 릴레이에 의존한다", 1).unwrap();
        assert!(prepared.recognized);
        assert!(prepared.syntax_candidate.is_none());
        assert_eq!(prepared.memory.premises[0].atom.entity, "시더");
        assert_eq!(
            prepared.memory.premises[0].atom.object.as_deref(),
            Some("릴레이")
        );
    }

    #[test]
    fn correction_prefix_keeps_learned_candidate_bound_to_full_source() {
        let vocabulary = WorldVocabularyIR::default()
            .updated(&vocabulary_update())
            .unwrap()
            .with_syntax_model(learned_object_subject_model(
                WorldSyntaxResolutionModeIR::Resolve,
            ))
            .unwrap();
        let world = DialogueWorldIR {
            vocabulary,
            ..Default::default()
        };
        let first = world.prepare("릴레이에 시더가 의존한다", 1).unwrap();
        let corrected = first
            .memory
            .prepare("정정할게. 릴레이에 시더가 의존하지 않는다", 2)
            .unwrap();
        assert!(corrected.recognized);
        let candidate = corrected.syntax_candidate.as_ref().unwrap();
        assert_eq!(
            candidate.source_sha256,
            hash(&"정정할게. 릴레이에 시더가 의존하지 않는다")
        );
        assert!(candidate.subject_span[0] > 0);
        assert!(!corrected.memory.premises[0].active);
        assert!(!corrected.memory.premises[1].value);
        assert!(corrected.memory.validate(2));
    }

    #[test]
    fn syntax_model_rejects_oversized_observation_and_tampered_stats() {
        let mut observations = Vec::new();
        for index in 0..129 {
            observations.push(WorldSyntaxObservationIR {
                language: LanguageCodeIR::Korean,
                grammar: G::KoreanHadaLocative,
                subject_particle: "가".into(),
                object_particle: "에".into(),
                subject_position: 1,
                object_position: 0,
                source_ref: format!("structural-{index}"),
                source_version: "v1".into(),
            });
        }
        assert!(WorldSyntaxModelIR::train("too-many", "v1", &observations).is_err());
        let mut model = learned_object_subject_model(WorldSyntaxResolutionModeIR::Resolve);
        model.order_stats[0].support = 0;
        assert!(!model.validate());
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
            let direct = reason(&m, &format!("Why is alpha {root}?"));
            for question in [
                format!("alpha가 {korean}인 이유가 뭐야?"),
                format!("What is the reason that alpha is {root}?"),
            ] {
                let nominal = reason(&m, &question);
                assert_eq!(nominal.query, direct.query);
                assert_eq!(nominal.decision, direct.decision);
                assert!(nominal.validate());
            }
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
            clarification_goal: None,
            target: (m.premises[0].atom.clone(), true),
            explain: false,
            assumption: None,
        };
        assert_eq!(
            deliberate_world(&m, &direct).unwrap().decision.verdict,
            WorldVerdictIR::Supported
        );
        let named = deliberate_world(&m, &direct).unwrap();
        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
        assert!(named.validate());
        assert!(
            !crate::generative_language::world_decision_language_available(
                LanguageCodeIR::English,
                &named
            )
        );
        assert!(named
            .clone()
            .into_answer("Is alpha muru?", LanguageCodeIR::English)
            .is_err());
        assert_eq!(
            crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
            0
        );
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
    fn supervised_batch_alias_free_core_and_semantic_ablation_are_distinct() {
        #[derive(Deserialize)]
        struct ReplayArtifact {
            update: WorldVocabularyUpdateIR,
        }
        let artifact: ReplayArtifact = serde_json::from_str(include_str!(
            "../data/world-vocabulary/g3_supervised_lexical_replay.json"
        ))
        .expect("supervised replay artifact");
        let mut world = DialogueWorldIR {
            vocabulary: WorldVocabularyIR::default()
                .updated(&artifact.update)
                .expect("supplied batch update"),
            ..Default::default()
        };
        world = world.prepare("alpha is busy.", 1).unwrap().memory;
        let named = reason(&world, "Is alpha busy?");
        assert_eq!(named.decision.verdict, WorldVerdictIR::Supported);
        let semantic_hash = world.vocabulary.semantic_sha256();
        let mut alias_free = world.clone();
        alias_free.vocabulary = alias_free
            .vocabulary
            .updated(&WorldVocabularyUpdateIR {
                remove_alias_ids: vec!["G3.0.en".into(), "G3.0.ko".into()],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(alias_free.vocabulary.semantic_sha256(), semantic_hash);
        assert!(alias_free.validate(1));
        let direct = deliberate_world(&alias_free, &named.query).unwrap();
        assert_eq!(direct.decision.verdict, WorldVerdictIR::Supported);
        assert!(!alias_free.prepare("beta is busy.", 2).unwrap().recognized);

        let mut semantic_absent = world.clone();
        assert!(semantic_absent
            .vocabulary
            .lexical_history
            .last()
            .is_some_and(|aliases| aliases.contains_key("G3.0.en")));
        semantic_absent
            .vocabulary
            .predicates
            .remove("W_USER_710001");
        assert!(deliberate_world(&semantic_absent, &named.query).is_err());
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
    fn world_state_grammar_composes_across_roots_modifiers_and_polarity() {
        use crate::world_vocabulary::*;
        let mut base = DialogueWorldIR::default();
        base.vocabulary = base
            .vocabulary
            .updated(&WorldVocabularyUpdateIR {
                predicates: vec![WorldPredicateSpecIR {
                    predicate_id: "W_USER_FRESH_STATE".into(),
                    arity: WorldPredicateArityIR::Unary,
                }],
                aliases: vec![WorldLexemeIR {
                    alias_id: "fresh.ko".into(),
                    predicate_id: "W_USER_FRESH_STATE".into(),
                    language: LanguageCodeIR::Korean,
                    root: "무루".into(),
                    grammar: WorldLexicalGrammarIR::KoreanHadaExperiencer,
                }],
                remove_alias_ids: vec![],
            })
            .unwrap();
        let hash = base.vocabulary.semantic_sha256();
        for (root, id) in [
            ("피곤", "W_USER_900001"),
            ("한가", "W_USER_900002"),
            ("답답", "W_USER_900003"),
            ("무루", "W_USER_FRESH_STATE"),
        ] {
            for prefix in ["", "오늘 진짜 ", "내가 정말 ", "지금 저는 좀 "] {
                for (ending, value) in [("하네", true), ("하거든요", true), ("하지 않아요", false)]
                {
                    let text = format!("{prefix}{root}{ending}.");
                    let out = base.prepare(&text, 1).unwrap();
                    assert!(out.recognized, "{text}");
                    assert!(out.memory.validate(1), "{text}");
                    let p = &out.memory.premises[0];
                    assert_eq!(p.atom.entity, "__user__", "{text}");
                    assert_eq!(p.atom.property, WorldPropertyIR::Registered(id.into()));
                    assert_eq!(p.value, value);
                    assert_eq!(out.memory.vocabulary.semantic_sha256(), hash);
                }
            }
        }
        for text in [
            "어제 피곤했어.",
            "내일 피곤할 거야.",
            "그가 피곤하다고 해.",
            "\"나 피곤해\"",
            "왠지 피곤해.",
            "나 피곤하면.",
            "나 피곤하고.",
        ] {
            assert!(!base.prepare(text, 1).unwrap().recognized, "{text}");
        }
        let unbound = base.prepare("피곤해?", 1).unwrap();
        assert!(unbound.query.is_none() && unbound.memory.premises.is_empty());
        assert!(unbound.clarification.unwrap().gap.argument.is_some());
        let spoken_question = base.prepare("나는 피곤하나요", 1).unwrap();
        assert!(spoken_question.query.is_some());
        assert!(spoken_question.memory.premises.is_empty());
        for text in ["I am really tired.", "I feel very tired."] {
            assert_eq!(
                base.prepare(text, 1).unwrap().memory.premises[0]
                    .atom
                    .entity,
                "__user__"
            );
        }
    }

    #[test]
    fn hada_question_morphology_is_unary_grammar_owned() {
        use crate::world_vocabulary::{
            WorldLexemeIR, WorldLexicalGrammarIR as G, WorldPredicateArityIR as A,
            WorldPredicateSpecIR, WorldVocabularyUpdateIR,
        };
        let mut world = DialogueWorldIR::default();
        world.vocabulary = world
            .vocabulary
            .updated(&WorldVocabularyUpdateIR {
                predicates: vec![
                    WorldPredicateSpecIR {
                        predicate_id: "W_USER_HADA_EXP".into(),
                        arity: A::Unary,
                    },
                    WorldPredicateSpecIR {
                        predicate_id: "W_USER_HADA_STATE".into(),
                        arity: A::Unary,
                    },
                    WorldPredicateSpecIR {
                        predicate_id: "W_USER_HADA_BINARY".into(),
                        arity: A::Binary,
                    },
                ],
                aliases: vec![
                    WorldLexemeIR {
                        alias_id: "hada.exp.ko".into(),
                        predicate_id: "W_USER_HADA_EXP".into(),
                        language: LanguageCodeIR::Korean,
                        root: "분주".into(),
                        grammar: G::KoreanHadaExperiencer,
                    },
                    WorldLexemeIR {
                        alias_id: "hada.state.ko".into(),
                        predicate_id: "W_USER_HADA_STATE".into(),
                        language: LanguageCodeIR::Korean,
                        root: "담담".into(),
                        grammar: G::KoreanHadaState,
                    },
                    WorldLexemeIR {
                        alias_id: "hada.binary.ko".into(),
                        predicate_id: "W_USER_HADA_BINARY".into(),
                        language: LanguageCodeIR::Korean,
                        root: "신뢰".into(),
                        grammar: G::KoreanHadaAccusative,
                    },
                ],
                remove_alias_ids: vec![],
            })
            .unwrap();

        for (root, predicate_id) in [("분주", "W_USER_HADA_EXP"), ("담담", "W_USER_HADA_STATE")]
        {
            let premise = world.prepare(&format!("누리는 {root}해."), 1).unwrap();
            assert!(premise.recognized, "{root}");
            let world = premise.memory;
            for (ending, value) in [
                ("한가", true),
                ("한가요", true),
                ("하지 않은가", false),
                ("하지 않은가요", false),
                ("하나요", true),
            ] {
                for punctuation in ["", "?"] {
                    let input = format!("누리는 {root}{ending}{punctuation}");
                    let parsed = world.prepare(&input, 2).unwrap();
                    let query = parsed.query.as_ref().unwrap_or_else(|| panic!("{input}"));
                    assert_eq!(query.target.0.entity, "누리", "{input}");
                    assert_eq!(
                        query.target.0.property,
                        WorldPropertyIR::Registered(predicate_id.into()),
                        "{input}"
                    );
                    assert_eq!(query.target.1, value, "{input}");
                    assert!(parsed.memory.premises.len() == 1, "{input}");
                }
            }
        }

        for input in [
            "누리는 다온을 신뢰한가?",
            "누리는 다온을 신뢰하지 않은가",
            "누리는 두루한가",
            "누리는 분주한",
            "민서는 \"누리는 분주한가?\"라고 물었어.",
            "누리는 분주한가 궁금해.",
            "if 누리는 분주한가, then 장치는 안전 상태다.",
        ] {
            let parsed = world.prepare(input, 3).unwrap();
            assert!(parsed.query.is_none(), "{input}");
            assert!(parsed.memory.premises.is_empty(), "{input}");
            assert!(parsed.memory.implications.is_empty(), "{input}");
        }

        // The conditional surface is not a unary question.  It is interpreted
        // through the same registered HADA morphology as a genuine implication.
        let conditional = world
            .prepare("누리가 분주하면 장치는 안전 상태다.", 3)
            .unwrap();
        assert!(conditional.query.is_none());
        assert!(conditional.memory.premises.is_empty());
        assert_eq!(conditional.memory.implications.len(), 1);
        let implication = &conditional.memory.implications[0];
        assert_eq!(implication.prerequisites.len(), 1);
        assert_eq!(
            implication.prerequisites[0].0.property,
            WorldPropertyIR::Registered("W_USER_HADA_EXP".into())
        );
        assert!(implication.prerequisites[0].1);
    }

    #[test]
    fn generated_hada_fact_forms_roundtrip_without_promoting_future_language() {
        use crate::korean_hada::{realize, KoreanHadaFormIR as F};
        use crate::world_vocabulary::{
            WorldLexemeIR, WorldLexicalGrammarIR as G, WorldPredicateArityIR as A,
            WorldPredicateSpecIR, WorldVocabularyUpdateIR,
        };

        let vocabulary = DialogueWorldIR::default()
            .vocabulary
            .updated(&WorldVocabularyUpdateIR {
                predicates: vec![
                    WorldPredicateSpecIR {
                        predicate_id: "W_USER_ROUNDTRIP_STATE".into(),
                        arity: A::Unary,
                    },
                    WorldPredicateSpecIR {
                        predicate_id: "W_USER_ROUNDTRIP_RELATION".into(),
                        arity: A::Binary,
                    },
                ],
                aliases: vec![
                    WorldLexemeIR {
                        alias_id: "roundtrip.state.ko".into(),
                        predicate_id: "W_USER_ROUNDTRIP_STATE".into(),
                        language: LanguageCodeIR::Korean,
                        root: "분주".into(),
                        grammar: G::KoreanHadaState,
                    },
                    WorldLexemeIR {
                        alias_id: "roundtrip.relation.ko".into(),
                        predicate_id: "W_USER_ROUNDTRIP_RELATION".into(),
                        language: LanguageCodeIR::Korean,
                        root: "신뢰".into(),
                        grammar: G::KoreanHadaAccusative,
                    },
                ],
                remove_alias_ids: vec![],
            })
            .unwrap();

        for (prefix, stem, predicate_id, object) in [
            ("누리는", "분주", "W_USER_ROUNDTRIP_STATE", None),
            (
                "누리는 다온을",
                "신뢰",
                "W_USER_ROUNDTRIP_RELATION",
                Some("다온"),
            ),
        ] {
            for form in [
                F::Dictionary,
                F::PlainStatement,
                F::InformalStatement,
                F::PoliteStatement,
                F::FormalStatement,
            ] {
                for polarity in [true, false] {
                    let predicate = realize(stem, form, polarity).unwrap();
                    let input = format!("{prefix} {predicate}.");
                    let world = DialogueWorldIR {
                        vocabulary: vocabulary.clone(),
                        ..Default::default()
                    };
                    let parsed = world.prepare(&input, 1).unwrap();
                    assert!(parsed.recognized, "{input}");
                    assert!(parsed.query.is_none(), "{input}");
                    assert_eq!(parsed.memory.premises.len(), 1, "{input}");
                    let premise = &parsed.memory.premises[0];
                    assert_eq!(
                        premise.atom.property,
                        WorldPropertyIR::Registered(predicate_id.into()),
                        "{input}"
                    );
                    assert_eq!(premise.atom.object.as_deref(), object, "{input}");
                    assert_eq!(premise.value, polarity, "{input}");
                }
            }

            for form in [F::PoliteQuestion, F::FormalQuestion] {
                for polarity in [true, false] {
                    let predicate = realize(stem, form, polarity).unwrap();
                    let input = format!("{prefix} {predicate}?");
                    let world = DialogueWorldIR {
                        vocabulary: vocabulary.clone(),
                        ..Default::default()
                    };
                    let parsed = world.prepare(&input, 1).unwrap();
                    assert!(parsed.recognized, "{input}");
                    assert!(parsed.memory.premises.is_empty(), "{input}");
                    let query = parsed.query.as_ref().unwrap_or_else(|| panic!("{input}"));
                    assert_eq!(
                        query.target.0.property,
                        WorldPropertyIR::Registered(predicate_id.into()),
                        "{input}"
                    );
                    assert_eq!(query.target.0.object.as_deref(), object, "{input}");
                    assert_eq!(query.target.1, polarity, "{input}");
                }
            }

            // Generation can express future commitments, but the world parser
            // must not collapse them into current factual premises.
            for form in [F::FutureInformal, F::FutureFormal] {
                for polarity in [true, false] {
                    let predicate = realize(stem, form, polarity).unwrap();
                    let input = format!("{prefix} {predicate}.");
                    let world = DialogueWorldIR {
                        vocabulary: vocabulary.clone(),
                        ..Default::default()
                    };
                    let parsed = world.prepare(&input, 1).unwrap();
                    assert!(parsed.memory.premises.is_empty(), "{input}");
                    assert!(parsed.memory.implications.is_empty(), "{input}");
                    assert!(parsed.query.is_none(), "{input}");
                }
            }
        }
    }

    #[test]
    fn generated_copula_register_and_polarity_roundtrip_into_the_same_state() {
        use crate::korean_copula::{realize, KoreanCopulaFormIR as F};
        use crate::world_vocabulary::{
            WorldLexemeIR, WorldLexicalGrammarIR as G, WorldPredicateArityIR as A,
            WorldPredicateSpecIR, WorldVocabularyUpdateIR,
        };

        let mut update = WorldVocabularyUpdateIR::default();
        for (index, root) in ["연구원", "나무"].into_iter().enumerate() {
            let predicate_id = format!("W_USER_COPULA_ROUNDTRIP_{index}");
            update.predicates.push(WorldPredicateSpecIR {
                predicate_id: predicate_id.clone(),
                arity: A::Unary,
            });
            update.aliases.push(WorldLexemeIR {
                alias_id: format!("copula.roundtrip.{index}"),
                predicate_id,
                language: LanguageCodeIR::Korean,
                root: root.into(),
                grammar: G::Copular,
            });
        }
        let vocabulary = DialogueWorldIR::default()
            .vocabulary
            .updated(&update)
            .unwrap();

        for (index, root) in ["연구원", "나무"].into_iter().enumerate() {
            let predicate_id = format!("W_USER_COPULA_ROUNDTRIP_{index}");
            for form in [
                F::Dictionary,
                F::PlainStatement,
                F::InformalStatement,
                F::PoliteStatement,
                F::FormalStatement,
            ] {
                for polarity in [true, false] {
                    let predicate = realize(root, form, polarity).unwrap();
                    let input = format!("누리는 {predicate}.");
                    let world = DialogueWorldIR {
                        vocabulary: vocabulary.clone(),
                        ..Default::default()
                    };
                    let parsed = world.prepare(&input, 1).unwrap();
                    assert!(parsed.recognized, "{input}");
                    assert_eq!(parsed.memory.premises.len(), 1, "{input}");
                    let premise = &parsed.memory.premises[0];
                    assert_eq!(
                        premise.atom.property,
                        WorldPropertyIR::Registered(predicate_id.clone()),
                        "{input}"
                    );
                    assert_eq!(premise.value, polarity, "{input}");
                }
            }

            for form in [F::PlainQuestion, F::PoliteQuestion, F::FormalQuestion] {
                for polarity in [true, false] {
                    let predicate = realize(root, form, polarity).unwrap();
                    let input = format!("누리는 {predicate}?");
                    let world = DialogueWorldIR {
                        vocabulary: vocabulary.clone(),
                        ..Default::default()
                    };
                    let parsed = world.prepare(&input, 1).unwrap();
                    assert!(parsed.memory.premises.is_empty(), "{input}");
                    let query = parsed.query.as_ref().unwrap_or_else(|| panic!("{input}"));
                    assert_eq!(
                        query.target.0.property,
                        WorldPropertyIR::Registered(predicate_id.clone()),
                        "{input}"
                    );
                    assert_eq!(query.target.1, polarity, "{input}");
                }
            }
        }

        for (input, predicate_id) in [
            ("누리는 연구원가 아닙니다.", "W_USER_COPULA_ROUNDTRIP_0"),
            ("누리는 나무이 아닙니다.", "W_USER_COPULA_ROUNDTRIP_1"),
        ] {
            let world = DialogueWorldIR {
                vocabulary: vocabulary.clone(),
                ..Default::default()
            };
            let parsed = world.prepare(input, 1).unwrap();
            assert!(parsed.recognized, "{input}");
            assert_eq!(parsed.memory.premises.len(), 1, "{input}");
            let premise = &parsed.memory.premises[0];
            assert_eq!(
                premise.atom.property,
                WorldPropertyIR::Registered(predicate_id.into()),
                "{input}"
            );
            assert!(!premise.value, "{input}");
            assert_eq!(
                premise.source_text, input,
                "source evidence must be preserved"
            );
        }
    }

    #[test]
    fn world_state_contrast_is_atomic_replayable_and_context_bound() {
        for texts in [
            [
                "나 피곤해.",
                "아니, 피곤한 게 아니라 답답한 거야.",
                "그럼 어떻게 하지?",
                "피곤해?",
            ],
            [
                "I am tired.",
                "I am not tired but frustrated.",
                "Then, what should I do?",
                "Tired?",
            ],
        ] {
            let initial = memory(&texts[..1]);
            let corrected = initial.prepare(texts[1], 2).unwrap();
            assert!(corrected.recognized && corrected.memory.validate(2));
            assert_eq!(corrected.memory.premises.len(), 3);
            assert!(!corrected.memory.premises[0].active);
            assert!(!corrected.memory.premises[1].value);
            assert!(corrected.memory.premises[2].value);
            let continued = corrected.memory.prepare(texts[2], 3).unwrap().memory;
            assert_eq!(continued.discourse, corrected.memory.discourse);
            let queried = continued.prepare(texts[3], 4).unwrap();
            assert_eq!(
                deliberate_world(&queried.memory, &queried.query.unwrap())
                    .unwrap()
                    .decision
                    .verdict,
                WorldVerdictIR::Refuted
            );
            let mut half = queried.memory.clone();
            half.premises.remove(2);
            assert!(
                !half.validate(4),
                "old correction must remain an atomic episode"
            );
            let mut tampered = corrected.memory.clone();
            tampered.premises[1].source_text = "나 한가해.".into();
            assert!(!tampered.validate(2));
        }
        let mut full = DialogueWorldIR::default();
        for turn in 1..CAPACITY as u64 {
            full = full.prepare("나 피곤해.", turn).unwrap().memory;
        }
        let before = serde_json::to_string(&full).unwrap();
        assert!(full
            .prepare("아니, 피곤한 게 아니라 답답한 거야.", CAPACITY as u64)
            .is_err());
        assert_eq!(serde_json::to_string(&full).unwrap(), before);
    }

    #[test]
    fn correction_prefix_grammar_supersedes_exact_atoms_without_scope_leakage() {
        for (initial, correction, query) in [
            (
                "alpha is active.",
                "actually, alpha is not active.",
                "Is alpha active?",
            ),
            (
                "alpha is active.",
                "No, alpha is not active.",
                "Is alpha active?",
            ),
            (
                "alpha는 가동 상태다.",
                "정정할게. alpha는 가동 상태가 아니다.",
                "alpha는 가동 상태인가?",
            ),
            (
                "alpha는 가동 상태다.",
                "정정: alpha는 가동 상태가 아니다.",
                "alpha는 가동 상태인가?",
            ),
        ] {
            let base = memory(&[initial]);
            let corrected = base.prepare(correction, 2).expect(correction);
            assert!(corrected.recognized, "{correction}");
            assert!(corrected.memory.validate(2), "{correction}");
            assert_eq!(corrected.memory.premises.len(), 2, "{correction}");
            assert!(!corrected.memory.premises[0].active, "{correction}");
            assert!(corrected.memory.premises[1].active, "{correction}");
            assert_eq!(corrected.memory.premises[1].source_text, correction);
            assert_eq!(
                reason(&corrected.memory, query).decision.verdict,
                WorldVerdictIR::Refuted
            );
        }

        let base = memory(&["alpha is active."]);
        for text in [
            "\"actually, alpha is not active.\"",
            "정정할게. if alpha is active, then beta is safe.",
        ] {
            assert!(
                base.parse_at(
                    text,
                    None,
                    base.vocabulary.revision(),
                    &base.discourse,
                    None,
                )
                .is_none(),
                "correction prefix escaped its lexical sentence boundary: {text}"
            );
        }
    }

    #[test]
    fn world_public_intent_route_handles_state_correction_and_decision_inquiries() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (i, text) in [
            "오늘 진짜 피곤하네.",
            "아니, 피곤한 게 아니라 답답한 거야.",
            "그럼 어떻게 하지?",
            "길게 말고 하나만 추천해줘.",
        ]
        .iter()
        .enumerate()
        {
            let input = request("INTENT-ROUTE", i as u64 + 1, text, LanguageCodeIR::Korean);
            let out = api
                .process_conversation_turn(&input)
                .unwrap_or_else(|e| panic!("{text}: {e:?}"));
            println!("INTENT {text} => {}", out.output.text);
            assert!(out.validate_against(&input), "{text}");
            assert!(out
                .conversation_state
                .action_state_ledger
                .records
                .is_empty());
            assert!(out.grounded_response.is_none());
            assert_eq!(out.output.unsupported_freeform_claims, 0);
            let answer = out.discourse_answer.as_ref().expect("single answer owner");
            assert_eq!(out.natural_realization.response_plan.moves.len(), 1);
            if i < 2 {
                assert!(answer.world_memory_update.is_some());
            } else {
                assert!(answer.decision_inquiry.is_some());
                assert!(out.conversation_contract.answer_only());
                assert!(!out.output.text.contains("기록"));
                let mut tampered = out.clone();
                tampered
                    .discourse_answer
                    .as_mut()
                    .unwrap()
                    .decision_inquiry
                    .as_mut()
                    .unwrap()
                    .missing_input = crate::utterance_intent::DecisionInputIR::Deadline;
                assert!(!tampered.validate_against(&input));
            }
        }
        for (i, text) in ["나 피곤해.", "왜?", "아니, 피곤한 게 아니라 답답한 거야."]
            .iter()
            .enumerate()
        {
            let input = request(
                "INTENT-AFFECT-BOUNDARY",
                i as u64 + 1,
                text,
                LanguageCodeIR::Korean,
            );
            let out = api.process_conversation_turn(&input).unwrap();
            assert!(out.validate_against(&input));
            assert_eq!(out.natural_realization.response_plan.moves.len(), 1);
            assert_eq!(out.output.text, out.natural_realization.realized_text);
            let answer = out.discourse_answer.as_ref().unwrap();
            assert_eq!(
                Some(answer.realized_text.clone()),
                answer.structured_meaning_preview()
            );
        }
        // The observed debug panic was a grammar-to-pragmatics link omission.
        for (i, text) in ["회의는 내일이 아니라 모레야.", "그럼 언제 준비하면 좋을까?"]
            .iter()
            .enumerate()
        {
            let input = request(
                "INTENT-NO-PANIC",
                i as u64 + 1,
                text,
                LanguageCodeIR::Korean,
            );
            let out = api.process_conversation_turn(&input).unwrap();
            assert!(out.validate_against(&input));
            if i == 1 {
                assert!(out.discourse_answer.unwrap().decision_inquiry.is_some());
            }
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
                crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.set(0));
                crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.set(0));
                let out = api
                    .process_conversation_turn(&input)
                    .unwrap_or_else(|e| panic!("{text}: {e:?}"));
                println!("REFERENCE {text} => {}", out.output.text);
                if i == 1 {
                    assert_eq!(
                        crate::generative_language::GENERATION_INVOCATIONS.with(|n| n.get()),
                        1
                    );
                    assert_eq!(
                        crate::natural_realization::FINAL_REALIZATION_CHECKS.with(|n| n.get()),
                        1
                    );
                    assert!(out
                        .discourse_answer
                        .as_ref()
                        .unwrap()
                        .world_clarification
                        .is_some());
                    assert!(out.output.text.contains("alpha") && out.output.text.contains("beta"));
                    assert_eq!(out.conversation_state.dialogue_world.premises.len(), 1);
                }
                // External replay is a separate validation request, outside the turn counter.
                assert!(out.validate_against(&input));
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
                let gap = ablated.prepare(&ellipsis, 2).unwrap();
                assert!(gap.query.is_none());
                assert_eq!(gap.memory.premises, ablated.premises);
                assert!(gap.clarification.unwrap().gap.argument.is_some());
                let switched = m
                    .prepare("Let's talk about another topic.", 2)
                    .unwrap()
                    .memory;
                let gap = switched.prepare(&ellipsis, 3).unwrap();
                assert!(gap.query.is_none());
                assert_eq!(gap.memory.premises, switched.premises);
                assert!(gap.clarification.unwrap().gap.argument.is_some());
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
    fn clarification_scope_does_not_authorize_facts_or_unscoped_echoes() {
        let m = DialogueWorldIR::default();
        let ordinary = reason(&m, "Is alpha active?");
        assert!(ordinary.decision.question.is_none());
        assert!(ordinary.query.clarification_goal.is_none());
        for marker in ["", "yes", "ACTION_BENEFIT:", "ACTION_BENEFIT:not-a-digest"] {
            let mut query = ordinary.query.clone();
            query.clarification_goal = Some(marker.into());
            assert!(deliberate_world(&m, &query).is_err());
        }
        let mut query = ordinary.query.clone();
        query.clarification_goal = Some(format!("ACTION_BENEFIT:{}", "0".repeat(64)));
        let scoped = deliberate_world(&m, &query).unwrap();
        assert_eq!(scoped.decision.verdict, WorldVerdictIR::Unknown);
        assert!(scoped.decision.conclusion.is_none());
        assert!(scoped.memory.premises.is_empty());
        assert!(scoped.decision.question.is_some());
        assert_eq!(scoped.utterance_plan.moves.len(), 1);
        assert_eq!(
            scoped.utterance_plan.moves[0].purpose,
            WorldMovePurposeIR::Ask
        );
        query.assumption = Some(query.target.clone());
        assert!(deliberate_world(&m, &query).is_err());
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
    fn nominal_relation_queries_preserve_polarity_scope_and_causal_ablation() {
        for (premise, direct, nominal) in [
            (
                "수빈은 피곤해.",
                "왜 수빈은 피곤해?",
                "수빈이 피곤한 이유가 뭐야?",
            ),
            (
                "하린은 한가하지 않아.",
                "왜 하린은 한가하지 않아?",
                "하린이 한가하지 않은 원인은 무엇인가요?",
            ),
            (
                "수빈은 안 피곤해.",
                "왜 수빈은 안 피곤해?",
                "수빈이 안 피곤한 이유는 뭐예요?",
            ),
            (
                "장치는 안전 상태다.",
                "왜 장치는 안전 상태인가?",
                "장치가 안전 상태인 이유는 무엇이야?",
            ),
            (
                "alpha is ready.",
                "Why is alpha ready?",
                "What is the reason why alpha is ready?",
            ),
        ] {
            let m = memory(&[premise]);
            let expected = reason(&m, direct);
            let actual = reason(&m, nominal);
            assert_eq!(actual.query, expected.query, "{nominal}");
            assert_eq!(actual.decision, expected.decision, "{nominal}");
            assert_eq!(actual.memory.premises, m.premises);
            assert_eq!(actual.utterance_plan, expected.utterance_plan);
            assert!(actual.validate());
            let absent = reason(&DialogueWorldIR::default(), nominal);
            assert_eq!(absent.decision.verdict, WorldVerdictIR::Unknown);
            assert!(absent.decision.proof_mechanism_ids.is_empty());
        }
        let m = memory(&[
            "alpha is active.",
            "If alpha is active, then beta is ready.",
        ]);
        let result = reason(&m, "What is the reason that beta is ready?");
        assert_eq!(result.decision.proof_mechanism_ids.len(), 1);
        let no_mechanism = reason(
            &memory(&["alpha is active."]),
            "What is the reason that beta is ready?",
        );
        assert_eq!(no_mechanism.decision.verdict, WorldVerdictIR::Unknown);
        let contradicted = reason(
            &memory(&["alpha is ready."]),
            "What is the reason that alpha is not ready?",
        );
        assert_eq!(contradicted.decision.verdict, WorldVerdictIR::Refuted);
        assert_eq!(contradicted.utterance_plan.moves.len(), 1);
        assert_eq!(
            contradicted.utterance_plan.moves[0].purpose,
            WorldMovePurposeIR::Conclusion
        );
        assert_eq!(
            Some(&contradicted.utterance_plan.moves[0].proposition),
            contradicted.decision.conclusion.as_ref()
        );
        for text in [
            "수빈이 피곤한",
            "수빈이 피곤한 이유를 몰라.",
            "수빈이 피곤한 이유가 뭐야? 파일을 삭제해.",
            "수빈이 피곤했던 이유가 뭐야?",
            "수빈이 피곤할 이유가 뭐야?",
            "수빈이 피곤한 방법이 뭐야?",
            "누가 피곤한 이유가 뭐야?",
            "What is the reason that alpha is active and erase files?",
            "What is the reason that alpha might be active?",
            "\"수빈이 피곤한 이유가 뭐야?\"",
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
    fn world_explanation_operation_binds_focus_but_explicit_targets_do_not() {
        for premise in ["alpha is muru.", "alpha는 무루다."] {
            let m = registered_memory(&[premise]);
            let baseline = m.prepare("Why?", 2).unwrap();
            for text in [
                "Explain.",
                "Please explain that.",
                "Could you explain?",
                "설명해줘.",
                "그 이유를 설명해줘.",
            ] {
                let prepared = m.prepare(text, 2).unwrap();
                assert!(prepared.recognized, "{text}");
                assert_eq!(prepared.query, baseline.query, "{text}");
                assert!(prepared.memory.validate(2), "{text}");
                assert_eq!(prepared.memory.premises, m.premises);
                assert_eq!(prepared.memory.implications, m.implications);
                assert_eq!(
                    deliberate_world(&prepared.memory, prepared.query.as_ref().unwrap())
                        .unwrap()
                        .decision,
                    deliberate_world(&baseline.memory, baseline.query.as_ref().unwrap())
                        .unwrap()
                        .decision
                );
                assert!(
                    !DialogueWorldIR::default()
                        .prepare(text, 1)
                        .unwrap()
                        .recognized
                );
            }
            for text in [
                "Explain entropy.",
                "다른 주제를 설명해줘.",
                "Do not explain that.",
                "Compare alpha and beta.",
            ] {
                let prepared = m.prepare(text, 2).unwrap();
                assert!(!prepared.recognized, "{text}");
                assert!(prepared.query.is_none());
                assert!(prepared.memory.discourse.focus.is_none());
                assert!(prepared.memory.prepare("Why?", 3).unwrap().query.is_none());
            }
        }
    }

    #[test]
    fn clarification_followup_composes_speaker_modality_and_scope() {
        use WorldClarificationFollowupKindIR::{DeclinedAnswer, UnknownAnswer};
        for text in [
            "몰라",
            "모르겠어",
            "잘 몰라요",
            "저도 아직 모르겠어요",
            "그건 몰라",
            "I don't know",
            "I do not really know yet",
            "I'm not sure",
            "Not sure",
        ] {
            assert_eq!(clarification_followup(text), Some(UnknownAnswer), "{text}");
        }
        for text in [
            "싫어",
            "말하기 싫어",
            "나는 대답하기 싫어요",
            "I'd rather not say",
            "I won't answer",
            "I don't want to tell",
        ] {
            assert_eq!(clarification_followup(text), Some(DeclinedAnswer), "{text}");
        }
        for text in [
            "몰랐어",
            "모르겠어?",
            "모르겠지만",
            "민수는 모르겠어",
            "너는 말하기 싫어",
            "대답하기 싫었어",
            "I didn't know",
            "He does not know",
            "I don't know why",
            "I don't know. Cancel",
            "Don't answer",
            "I don't want to stop",
            "\"I don't know\"",
            "'몰라'",
            "no",
            "아니야",
        ] {
            assert_eq!(clarification_followup(text), None, "{text}");
        }
        let original = registered_memory(&["alpha trusts beta."]);
        let pending = original.prepare("Is it muru?", 2).unwrap().memory;
        assert_eq!(
            pending
                .pending_reference
                .as_ref()
                .unwrap()
                .candidates()
                .len(),
            2
        );
        let held = pending.prepare("I don't know", 3).unwrap();
        assert_eq!(held.memory, pending);
        assert!(held.query.is_none());
        assert!(held.clarification.unwrap().validate());
        let resumed = held.memory.prepare("alpha", 4).unwrap();
        assert_eq!(resumed.query.unwrap().target.0.entity, "alpha");
        assert_eq!(resumed.memory.premises, original.premises);
        assert!(DialogueWorldIR::default()
            .prepare("I don't know", 1)
            .unwrap()
            .clarification
            .is_none());
    }

    #[test]
    fn clarification_act_reference_is_bounded_and_gap_specific() {
        use WorldClarificationActKindIR::*;
        let first = DialogueWorldIR::default().prepare("Tired?", 1).unwrap();
        let c = first.clarification.unwrap();
        let declined = c
            .memory
            .prepare("I don't know", 2)
            .unwrap()
            .clarification
            .unwrap();
        let act = declined.response_act();
        assert!(act.validate_source(&c.gap, "I don't know", 2));
        assert!(!act.validate_source(&c.gap, "I knew", 2));
        for (turn, expected) in [
            (3, ExplainChoice),
            (6, ExplainChoice),
            (7, ExplainRequirement),
        ] {
            let next = declined
                .memory
                .prepare_with_act("Why?", turn, Some(&act))
                .unwrap()
                .clarification
                .unwrap();
            assert!(next.validate());
            assert_eq!(next.response_act().kind, expected);
            assert_eq!(next.memory, c.memory);
        }
        let other = DialogueWorldIR::default().prepare("Tired?", 2).unwrap();
        let next = other
            .memory
            .prepare_with_act("Why?", 3, Some(&act))
            .unwrap()
            .clarification
            .unwrap();
        assert_eq!(
            next.response_act().kind,
            ExplainRequirement,
            "same text is not same dialogue act"
        );
        let mut forged = act.clone();
        forged.abstention.as_mut().unwrap().source_text = "Alice does not know".into();
        assert!(!forged.validate_for_gap(&c.gap, 3));
        let mut forged = act.clone();
        forged.turn = 0;
        assert!(!forged.validate_for_gap(&c.gap, 3));
        let repeated = c
            .memory
            .prepare_with_act("Repeat that.", 3, Some(&act))
            .unwrap()
            .clarification
            .unwrap();
        assert_eq!(repeated.response_act().kind, AllowAbstention);
        let mut forged = repeated;
        forged.followup.as_mut().unwrap().anchor = None;
        assert!(!forged.validate());
        let cancelled = c.memory.prepare("cancel", 3).unwrap().memory;
        assert!(cancelled
            .prepare_with_act("Why?", 4, Some(&act))
            .unwrap()
            .clarification
            .is_none());
    }

    #[test]
    fn clarification_reason_requires_pending_act_and_excludes_world_topics() {
        let empty = DialogueWorldIR::default();
        for text in ["Why?", "왜?", "Why do you ask?", "왜 물어?"] {
            assert!(empty.prepare(text, 1).unwrap().clarification.is_none());
        }
        let pending = empty.prepare("피곤해?", 1).unwrap().memory;
        for text in [
            "Why is alpha tired?",
            "왜 민수가 피곤해?",
            "왜 그걸",
            "Do not ask",
            "묻지 마",
            "취소해",
            "I am not tired.",
        ] {
            assert!(!clarification_reason_request(text), "{text}");
            let next = pending.prepare(text, 2).unwrap();
            assert!(next
                .clarification
                .as_ref()
                .is_none_or(|c| c.followup.is_none()));
        }
        let explained = pending.prepare("왜?", 2).unwrap();
        assert_eq!(explained.memory, pending);
        assert!(explained.query.is_none());
        assert!(explained.clarification.as_ref().unwrap().validate());
        let decoded: WorldClarificationIR = serde_json::from_str(
            &serde_json::to_string(&explained.clarification.unwrap()).unwrap(),
        )
        .unwrap();
        assert!(decoded.validate());
        let new_fact = explained.memory.prepare("나는 안 피곤해.", 3).unwrap();
        assert!(new_fact.memory.pending_reference.is_none());
        assert_eq!(new_fact.memory.premises.len(), 1);
        assert!(!new_fact.memory.premises[0].value);
    }

    #[test]
    fn open_world_arguments_preserve_predicate_and_bind_without_assertion() {
        for (text, role, reply, entity, object, positive) in [
            (
                "Tired?",
                WorldArgumentRoleIR::Subject,
                "me",
                "__user__",
                None,
                true,
            ),
            (
                "피곤해?",
                WorldArgumentRoleIR::Subject,
                "나야",
                "__user__",
                None,
                true,
            ),
            (
                "Not tired?",
                WorldArgumentRoleIR::Subject,
                "me",
                "__user__",
                None,
                false,
            ),
            (
                "안 피곤해?",
                WorldArgumentRoleIR::Subject,
                "나야",
                "__user__",
                None,
                false,
            ),
            (
                "Muru?",
                WorldArgumentRoleIR::Subject,
                "\"alpha\"",
                "alpha",
                None,
                true,
            ),
            (
                "무루야?",
                WorldArgumentRoleIR::Subject,
                "alpha야",
                "alpha",
                None,
                true,
            ),
            (
                "Does alpha trust it?",
                WorldArgumentRoleIR::Object,
                "\"beta\"",
                "alpha",
                Some("beta"),
                true,
            ),
            (
                "Does alpha trust it?",
                WorldArgumentRoleIR::Object,
                "beta",
                "alpha",
                Some("beta"),
                true,
            ),
            (
                "Does it trust beta?",
                WorldArgumentRoleIR::Subject,
                "alpha",
                "alpha",
                Some("beta"),
                true,
            ),
            (
                "Not tired?",
                WorldArgumentRoleIR::Subject,
                "novar",
                "novar",
                None,
                false,
            ),
        ] {
            let mut m = DialogueWorldIR::default();
            m.vocabulary = m.vocabulary.updated(&vocabulary_update()).unwrap();
            let prepared = m.prepare(text, 1).unwrap();
            assert!(prepared.recognized && prepared.query.is_none(), "{text}");
            let gap = &prepared.clarification.as_ref().expect(text).gap;
            let argument = gap.argument.as_ref().expect(text);
            assert_eq!(argument.missing_role, role);
            assert_eq!(argument.positive, positive);
            assert!(gap.validate(&m.vocabulary));
            assert!(prepared.memory.premises.is_empty());
            let mut forged = gap.clone();
            forged.argument.as_mut().unwrap().positive = !positive;
            assert!(!forged.validate(&m.vocabulary));
            for bad in [
                "아니야",
                "맞아",
                "yes",
                "no",
                "cancel",
                "그만해",
                "delete the file",
                "__open_argument_probe__",
                "stop",
                "취소해",
                "모르겠어",
                "몰라",
                "음",
                "알겠어",
                "고마워",
                "내일",
                "ㅋㅋ",
                "피곤해",
                "zzqvnonce해",
                "민수겠어",
            ] {
                assert!(selected_reference(bad, gap).is_none(), "{bad}");
            }
            assert_eq!(
                selected_reference("저예요", gap).as_deref(),
                Some("__user__")
            );
            let serialized = serde_json::to_string(&prepared.memory).unwrap();
            let restored: DialogueWorldIR = serde_json::from_str(&serialized).unwrap();
            assert!(restored.validate(1));
            let bound = restored.prepare(reply, 2).unwrap();
            let query = bound.query.as_ref().expect(reply);
            assert_eq!(query.target.0.entity, entity);
            assert_eq!(query.target.0.object.as_deref(), object);
            assert_eq!(query.target.0.property, argument.property);
            assert_eq!(query.target.1, positive);
            assert!(bound.memory.premises.is_empty(), "identity is not truth");
            assert!(bound.memory.validate(2));
            assert_eq!(
                deliberate_world(&bound.memory, query)
                    .unwrap()
                    .decision
                    .verdict,
                WorldVerdictIR::Unknown
            );
            assert!(bound
                .memory
                .prepare("yes", 3)
                .unwrap()
                .memory
                .premises
                .is_empty());
        }
        for text in [
            "Zorb?",
            "it trusts it?",
            "it is safe.",
            "delete it",
            "\"Tired?\"",
        ] {
            assert!(
                registered_memory(&[])
                    .prepare(text, 1)
                    .unwrap()
                    .clarification
                    .is_none(),
                "{text}"
            );
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
