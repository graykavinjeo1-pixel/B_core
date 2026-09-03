//! Claim-level evidence boundary for conversational realization.
//!
//! Claims are projected from typed semantic products.  The realized text is
//! never parsed back into authority, and a language-only report can never be
//! promoted to verified execution evidence.

use std::collections::BTreeSet;

use dockable_semantic_core::PlanIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::action_state::{
    ActionExecutionStatusIR, ActionSetQueryIR, ActionStateAnalysisIR, ActionStateLedgerIR,
    ActionStateRecordIR,
};
use crate::conditional_guard::ConditionalGuardEvaluationIR;
use crate::conversation::{
    DiscourseGroupUpdateIR, DiscourseTopicIR, TopicAnchoredReferenceIR, TopicTransitionIR,
};
use crate::discourse_qa::DiscourseAnswerIR;
use crate::discourse_relations::DialogueRelationAnswerIR;
use crate::epistemic::EpistemicLedgerIR;
use crate::language_knowledge::LanguageCodeIR;
use crate::temporal::TemporalAnswerIR;

pub const EVIDENCE_GROUNDED_REALIZATION_SCHEMA: &str = "B_CORE_EVIDENCE_GROUNDED_REALIZATION_IR_1";

const MAX_REALIZATION_CLAIMS: usize = 64;
const MAX_CLAIM_EVIDENCE: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GroundedClaimKindIR {
    PlanStatus,
    ActionSetEvaluation,
    LanguageReport,
    VerifiedExecution,
    AttributedDialogueRecord,
    TemporalRelation,
    DialogueRelation,
    ConditionalGuard,
    EvidenceAbsence,
    InteractionState,
    DiscourseGroupRevision,
    DiscourseTopicTransition,
    TopicAnchoredReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimSupportStatusIR {
    StructurallyGrounded,
    ReportedOnly,
    VerifiedEvidence,
    DerivedFromDialogueRecords,
    EvidenceAbsent,
    NonFactual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimEpistemicStatusIR {
    Planned,
    Reported,
    VerifiedObserved,
    Derived,
    Unknown,
    Interaction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundedClaimIR {
    pub claim_id: String,
    pub kind: GroundedClaimKindIR,
    pub proposition: String,
    pub epistemic_status: ClaimEpistemicStatusIR,
    pub support_status: ClaimSupportStatusIR,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub source_turns: Vec<u64>,
    pub verified: bool,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
}

impl GroundedClaimIR {
    fn validate(&self) -> bool {
        let evidence = self.evidence_refs.iter().collect::<BTreeSet<_>>();
        let turns = self.source_turns.iter().collect::<BTreeSet<_>>();
        let evidence_required = self.support_status != ClaimSupportStatusIR::NonFactual;
        let verified = self.support_status == ClaimSupportStatusIR::VerifiedEvidence;
        !self.claim_id.trim().is_empty()
            && !self.proposition.trim().is_empty()
            && self.evidence_refs.len() <= MAX_CLAIM_EVIDENCE
            && evidence.len() == self.evidence_refs.len()
            && turns.len() == self.source_turns.len()
            && !self.source_turns.is_empty()
            && self.source_turns.iter().all(|turn| *turn > 0)
            && (!evidence_required || !self.evidence_refs.is_empty())
            && self.verified == verified
            && !self.semantic_authority
            && !self.external_action_executed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceGroundedRealizationIR {
    pub schema: String,
    pub language: LanguageCodeIR,
    pub realized_text: String,
    pub claims: Vec<GroundedClaimIR>,
    pub unsupported_claims: usize,
    pub faithful: bool,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
    pub realization_sha256: String,
}

impl EvidenceGroundedRealizationIR {
    pub fn validate(&self) -> bool {
        let ids = self
            .claims
            .iter()
            .map(|claim| claim.claim_id.as_str())
            .collect::<BTreeSet<_>>();
        self.schema == EVIDENCE_GROUNDED_REALIZATION_SCHEMA
            && !self.realized_text.trim().is_empty()
            && !self.claims.is_empty()
            && self.claims.len() <= MAX_REALIZATION_CLAIMS
            && ids.len() == self.claims.len()
            && self.claims.iter().all(GroundedClaimIR::validate)
            && self.unsupported_claims == 0
            && self.faithful
            && !self.semantic_authority
            && !self.external_action_executed
            && self.realization_sha256 == realization_sha256(self)
    }
}

pub(crate) struct GroundedRealizationSources<'a> {
    pub language: LanguageCodeIR,
    pub realized_text: &'a str,
    pub turn_index: u64,
    pub plan: Option<&'a PlanIR>,
    pub action_analysis: &'a ActionStateAnalysisIR,
    pub action_ledger: &'a ActionStateLedgerIR,
    pub competing_outcome_reports: bool,
    pub epistemic_ledger: Option<&'a EpistemicLedgerIR>,
    pub discourse_group_update: Option<&'a DiscourseGroupUpdateIR>,
    pub topic_transition: Option<&'a TopicTransitionIR>,
    pub active_topic: Option<&'a DiscourseTopicIR>,
    pub topic_anchored_reference: Option<&'a TopicAnchoredReferenceIR>,
    pub discourse_answer: Option<&'a DiscourseAnswerIR>,
    pub dialogue_relation_answer: Option<&'a DialogueRelationAnswerIR>,
    pub temporal_answer: Option<&'a TemporalAnswerIR>,
    pub guard_evaluations: &'a [ConditionalGuardEvaluationIR],
    pub evidence_absence: bool,
    pub source_unsupported_claims: usize,
}

pub(crate) fn build_evidence_grounded_realization(
    source: GroundedRealizationSources<'_>,
) -> EvidenceGroundedRealizationIR {
    // A result-absence response is an epistemic boundary decision, not an
    // invitation for whichever lower-level analyzer happened to recognize a
    // plan, guard, report, or topic operation on the same turn to own the
    // claims.  Keep that decision authoritative so the realized answer cannot
    // simultaneously say "unknown" and project a stronger unrelated status.
    let mut claims = if source.competing_outcome_reports {
        conflicting_report_claims(source.epistemic_ledger, source.turn_index)
    } else if source.evidence_absence {
        vec![claim(
            0,
            GroundedClaimKindIR::EvidenceAbsence,
            "no verified execution result is recorded".to_string(),
            ClaimEpistemicStatusIR::Unknown,
            ClaimSupportStatusIR::EvidenceAbsent,
            absence_evidence(source.action_ledger),
            vec![source.turn_index],
        )]
    } else if let Some(update) = source
        .discourse_group_update
        .filter(|update| update.applied)
    {
        vec![discourse_group_update_claim(update, source.turn_index)]
    } else if let Some((transition, topic)) = source
        .topic_transition
        .filter(|transition| transition.applied)
        .zip(source.active_topic)
    {
        vec![topic_transition_claim(transition, topic, source.turn_index)]
    } else if let Some(reference) = source
        .topic_anchored_reference
        .filter(|reference| reference.applied)
    {
        let mut grounded = vec![topic_anchored_reference_claim(reference, source.turn_index)];
        if let Some(plan) = source.plan {
            grounded.push(plan_claim(plan, source.turn_index));
        }
        grounded
    } else if !source.guard_evaluations.is_empty() {
        guard_claims(source.guard_evaluations)
    } else if source.action_analysis.consumes_turn() {
        action_claims(
            source.action_analysis,
            source.action_ledger,
            source.turn_index,
        )
    } else if let Some(answer) = source.temporal_answer {
        temporal_claims(answer, source.turn_index)
    } else if let Some(answer) = source.dialogue_relation_answer {
        dialogue_relation_claims(answer, source.turn_index)
    } else if let Some(answer) = source.discourse_answer {
        discourse_claims(answer, source.turn_index)
    } else if let Some(plan) = source.plan {
        vec![plan_claim(plan, source.turn_index)]
    } else {
        vec![claim(
            0,
            GroundedClaimKindIR::InteractionState,
            "the response manages the current dialogue state".to_string(),
            ClaimEpistemicStatusIR::Interaction,
            ClaimSupportStatusIR::NonFactual,
            Vec::new(),
            vec![source.turn_index],
        )]
    };
    reseal_claim_ids(&mut claims);
    let mut realization = EvidenceGroundedRealizationIR {
        schema: EVIDENCE_GROUNDED_REALIZATION_SCHEMA.to_string(),
        language: source.language,
        realized_text: source.realized_text.to_string(),
        claims,
        unsupported_claims: source.source_unsupported_claims,
        faithful: source.source_unsupported_claims == 0,
        semantic_authority: false,
        external_action_executed: false,
        realization_sha256: String::new(),
    };
    realization.realization_sha256 = realization_sha256(&realization);
    realization
}

fn topic_anchored_reference_claim(
    reference: &TopicAnchoredReferenceIR,
    turn_index: u64,
) -> GroundedClaimIR {
    let mut evidence_refs = vec![
        reference.resolution_sha256.clone(),
        reference.topic_sha256.clone(),
        reference.membership_sha256.clone(),
        reference.group_id.clone(),
    ];
    evidence_refs.extend(reference.selected_member_keys.iter().cloned());
    claim(
        0,
        GroundedClaimKindIR::TopicAnchoredReference,
        format!(
            "topic {} group {} revision {} selected {} members through {:?}",
            reference.topic_id,
            reference.group_id,
            reference.group_revision,
            reference.selected_member_keys.len(),
            reference.selector
        ),
        ClaimEpistemicStatusIR::Derived,
        ClaimSupportStatusIR::StructurallyGrounded,
        evidence_refs,
        vec![turn_index],
    )
}

fn topic_transition_claim(
    transition: &TopicTransitionIR,
    topic: &DiscourseTopicIR,
    turn_index: u64,
) -> GroundedClaimIR {
    let mut evidence_refs = vec![
        transition.transition_sha256.clone(),
        topic.topic_sha256.clone(),
    ];
    if let Some(membership_sha256) = transition.anchor_membership_sha256.as_ref() {
        evidence_refs.push(membership_sha256.clone());
    }
    evidence_refs.sort();
    evidence_refs.dedup();
    claim(
        0,
        GroundedClaimKindIR::DiscourseTopicTransition,
        format!(
            "topic {} is active through {:?}",
            topic.topic_id, transition.kind
        ),
        ClaimEpistemicStatusIR::Derived,
        ClaimSupportStatusIR::StructurallyGrounded,
        evidence_refs,
        vec![turn_index],
    )
}

fn discourse_group_update_claim(
    update: &DiscourseGroupUpdateIR,
    turn_index: u64,
) -> GroundedClaimIR {
    let target = update.target_group_id.as_deref().unwrap_or("UNRESOLVED");
    claim(
        0,
        GroundedClaimKindIR::DiscourseGroupRevision,
        format!(
            "discourse group {target} revision {} has {} members after {:?}",
            update.revision,
            update.after_member_keys.len(),
            update.operation
        ),
        ClaimEpistemicStatusIR::Derived,
        ClaimSupportStatusIR::StructurallyGrounded,
        std::iter::once(update.update_sha256.clone())
            .chain(update.source_group_ids.iter().cloned())
            .collect(),
        vec![turn_index],
    )
}

fn plan_claim(plan: &PlanIR, turn_index: u64) -> GroundedClaimIR {
    claim(
        0,
        GroundedClaimKindIR::PlanStatus,
        format!(
            "plan {} is structurally validated with {} steps",
            plan.goal_id,
            plan.steps.len()
        ),
        ClaimEpistemicStatusIR::Planned,
        ClaimSupportStatusIR::StructurallyGrounded,
        vec![plan.plan_sha256.clone(), plan.goal_id.clone()],
        vec![turn_index],
    )
}

fn action_claims(
    analysis: &ActionStateAnalysisIR,
    ledger: &ActionStateLedgerIR,
    turn_index: u64,
) -> Vec<GroundedClaimIR> {
    let mut claims = Vec::new();
    let mut records = analysis
        .target_action_ids
        .iter()
        .filter_map(|id| ledger.record(id))
        .collect::<Vec<_>>();
    if records.is_empty() {
        records.extend(ledger.current_record());
    }
    for record in &records {
        // The plan remains preserved in the ledger, but after an outcome
        // report the response-relevant epistemic boundary is report versus
        // verified evidence. Projecting the older plan into every later
        // verification answer would conflate lifecycle axes.
        if record.reported_status.is_none() {
            claims.push(action_plan_claim(record, claims.len()));
        }
        if let Some(report) = ledger
            .language_report_history
            .iter()
            .rev()
            .find(|report| report.action_id == record.action_id)
        {
            let mut evidence = vec![
                report.action_id.clone(),
                report.report_id.clone(),
                report.report_sha256.clone(),
                format!("TURN:{}", report.turn_index),
            ];
            evidence.extend(report.evidence.iter().cloned());
            claims.push(claim(
                claims.len(),
                GroundedClaimKindIR::LanguageReport,
                format!(
                    "action {} was reported {:?}",
                    report.action_id, report.reported_status
                ),
                ClaimEpistemicStatusIR::Reported,
                ClaimSupportStatusIR::ReportedOnly,
                evidence,
                vec![report.turn_index],
            ));
        } else if let Some(report) = analysis
            .language_reports()
            .into_iter()
            .find(|report| report.action_id == record.action_id)
        {
            let mut evidence = vec![report.action_id.clone(), format!("TURN:{turn_index}")];
            evidence.extend(report.evidence.iter().cloned());
            claims.push(claim(
                claims.len(),
                GroundedClaimKindIR::LanguageReport,
                format!(
                    "action {} was reported {:?}",
                    report.action_id, report.reported_status
                ),
                ClaimEpistemicStatusIR::Reported,
                ClaimSupportStatusIR::ReportedOnly,
                evidence,
                vec![turn_index],
            ));
        }
        match record.execution_status {
            ActionExecutionStatusIR::NotObserved => claims.push(claim(
                claims.len(),
                GroundedClaimKindIR::EvidenceAbsence,
                format!(
                    "action {} has no verified execution result",
                    record.action_id
                ),
                ClaimEpistemicStatusIR::Unknown,
                ClaimSupportStatusIR::EvidenceAbsent,
                vec![record.action_id.clone()],
                vec![record.introduced_turn, turn_index],
            )),
            status => {
                let mut evidence = vec![record.action_id.clone()];
                evidence.extend(record.execution_evidence_ids.iter().cloned());
                claims.push(claim(
                    claims.len(),
                    GroundedClaimKindIR::VerifiedExecution,
                    format!("action {} has observed status {status:?}", record.action_id),
                    ClaimEpistemicStatusIR::VerifiedObserved,
                    ClaimSupportStatusIR::VerifiedEvidence,
                    evidence,
                    vec![record.introduced_turn, record.last_update_turn, turn_index],
                ));
            }
        }
    }
    if let Some(query) = analysis
        .set_query
        .as_ref()
        .filter(|query| query.predicate.is_some() && query.quantifier.is_some())
    {
        claims.push(action_set_evaluation_claim(
            query,
            ledger,
            turn_index,
            claims.len(),
        ));
    }
    if claims.is_empty() {
        claims.push(claim(
            0,
            GroundedClaimKindIR::EvidenceAbsence,
            "no action record can be bound to the request".to_string(),
            ClaimEpistemicStatusIR::Unknown,
            ClaimSupportStatusIR::EvidenceAbsent,
            vec!["ACTION_LEDGER:NO_MATCH".to_string()],
            vec![turn_index],
        ));
    }
    claims
}

fn conflicting_report_claims(
    ledger: Option<&EpistemicLedgerIR>,
    turn_index: u64,
) -> Vec<GroundedClaimIR> {
    let Some(ledger) = ledger else {
        return vec![
            claim(
                0,
                GroundedClaimKindIR::LanguageReport,
                "multiple attributed outcome reports conflict".to_string(),
                ClaimEpistemicStatusIR::Reported,
                ClaimSupportStatusIR::ReportedOnly,
                vec!["EPISTEMIC_LEDGER:CONFLICTING_REPORTS".to_string()],
                vec![turn_index],
            ),
            claim(
                1,
                GroundedClaimKindIR::EvidenceAbsence,
                "the conflicting reports do not establish a verified execution result".to_string(),
                ClaimEpistemicStatusIR::Unknown,
                ClaimSupportStatusIR::EvidenceAbsent,
                vec!["EPISTEMIC_LEDGER:NO_VERIFIED_RESULT".to_string()],
                vec![turn_index],
            ),
        ];
    };
    let mut records = ledger
        .records
        .iter()
        .filter(|record| record.introduced_turn == turn_index)
        .collect::<Vec<_>>();
    if records.len() < 2 {
        records = ledger.records.iter().collect();
    }
    let mut evidence = records
        .iter()
        .flat_map(|record| [record.belief_id.clone(), record.origin_referent_id.clone()])
        .collect::<Vec<_>>();
    evidence.extend(
        ledger
            .unresolved_conflicts
            .iter()
            .map(|conflict| format!("CONFLICT:{conflict}")),
    );
    if evidence.is_empty() {
        evidence.push("EPISTEMIC_LEDGER:CONFLICTING_REPORTS".to_string());
    }
    let mut turns = records
        .iter()
        .map(|record| record.introduced_turn)
        .collect::<Vec<_>>();
    turns.push(turn_index);
    vec![
        claim(
            0,
            GroundedClaimKindIR::LanguageReport,
            "multiple attributed outcome reports conflict".to_string(),
            ClaimEpistemicStatusIR::Reported,
            ClaimSupportStatusIR::ReportedOnly,
            evidence.clone(),
            turns.clone(),
        ),
        claim(
            1,
            GroundedClaimKindIR::EvidenceAbsence,
            "the conflicting reports do not establish a verified execution result".to_string(),
            ClaimEpistemicStatusIR::Unknown,
            ClaimSupportStatusIR::EvidenceAbsent,
            evidence,
            turns,
        ),
    ]
}

fn action_set_evaluation_claim(
    query: &ActionSetQueryIR,
    ledger: &ActionStateLedgerIR,
    turn_index: u64,
    index: usize,
) -> GroundedClaimIR {
    let mut evidence = vec![query.query_sha256.clone()];
    let mut turns = vec![turn_index];
    for action_id in &query.selected_action_ids {
        let Some(record) = ledger.record(action_id) else {
            continue;
        };
        evidence.push(record.action_id.clone());
        evidence.push(record.goal_id.clone());
        evidence.push(format!("PLAN_STATUS:{:?}", record.plan_status));
        evidence.push(format!("REPORTED_STATUS:{:?}", record.reported_status));
        evidence.push(format!("EXECUTION_STATUS:{:?}", record.execution_status));
        evidence.extend(record.execution_evidence_ids.iter().cloned());
        turns.push(record.introduced_turn);
        turns.push(record.last_update_turn);
    }
    evidence.sort();
    evidence.dedup();
    turns.sort_unstable();
    turns.dedup();
    claim(
        index,
        GroundedClaimKindIR::ActionSetEvaluation,
        format!(
            "action-set {:?} over {:?} evaluated {:?} for {} selected actions",
            query.quantifier,
            query.predicate,
            query.truth,
            query.selected_action_ids.len()
        ),
        ClaimEpistemicStatusIR::Derived,
        ClaimSupportStatusIR::DerivedFromDialogueRecords,
        evidence,
        turns,
    )
}

fn action_plan_claim(record: &ActionStateRecordIR, index: usize) -> GroundedClaimIR {
    claim(
        index,
        GroundedClaimKindIR::PlanStatus,
        format!(
            "action {} has plan status {:?}",
            record.action_id, record.plan_status
        ),
        ClaimEpistemicStatusIR::Planned,
        ClaimSupportStatusIR::StructurallyGrounded,
        vec![record.action_id.clone(), record.goal_id.clone()],
        vec![record.introduced_turn],
    )
}

fn discourse_claims(answer: &DiscourseAnswerIR, turn_index: u64) -> Vec<GroundedClaimIR> {
    if answer.claims.is_empty() {
        return vec![claim(
            0,
            GroundedClaimKindIR::EvidenceAbsence,
            format!("dialogue query has disposition {:?}", answer.disposition),
            ClaimEpistemicStatusIR::Unknown,
            ClaimSupportStatusIR::EvidenceAbsent,
            vec![format!("QUERY:{:?}", answer.query.kind)],
            vec![turn_index],
        )];
    }
    answer
        .claims
        .iter()
        .enumerate()
        .map(|(index, source_claim)| {
            let mut source_turns = answer
                .evidence
                .iter()
                .filter(|evidence| {
                    source_claim
                        .evidence_belief_ids
                        .contains(&evidence.belief_id)
                })
                .map(|evidence| evidence.introduced_turn)
                .collect::<Vec<_>>();
            let mut evidence_refs = source_claim.evidence_belief_ids.clone();
            let (epistemic_status, support_status) = if evidence_refs.is_empty() {
                source_turns.push(turn_index);
                evidence_refs.push(format!("QUERY:{:?}", answer.query.kind));
                (
                    ClaimEpistemicStatusIR::Unknown,
                    ClaimSupportStatusIR::EvidenceAbsent,
                )
            } else {
                (
                    ClaimEpistemicStatusIR::Derived,
                    ClaimSupportStatusIR::DerivedFromDialogueRecords,
                )
            };
            claim(
                index,
                GroundedClaimKindIR::AttributedDialogueRecord,
                format!("{}: {}", source_claim.subject, source_claim.value),
                epistemic_status,
                support_status,
                evidence_refs,
                source_turns,
            )
        })
        .collect()
}

fn temporal_claims(answer: &TemporalAnswerIR, turn_index: u64) -> Vec<GroundedClaimIR> {
    let mut claims = answer
        .relation_evidence
        .iter()
        .enumerate()
        .map(|(index, relation)| {
            claim(
                index,
                GroundedClaimKindIR::TemporalRelation,
                format!(
                    "{} {:?} {}",
                    relation.left_event_id, relation.kind, relation.right_event_id
                ),
                ClaimEpistemicStatusIR::Derived,
                ClaimSupportStatusIR::DerivedFromDialogueRecords,
                vec![
                    relation.relation_id.clone(),
                    relation.left_event_id.clone(),
                    relation.right_event_id.clone(),
                ],
                vec![relation.introduced_turn],
            )
        })
        .collect::<Vec<_>>();
    if claims.is_empty() && !answer.event_evidence.is_empty() {
        claims.extend(
            answer
                .event_evidence
                .iter()
                .enumerate()
                .map(|(index, event)| {
                    claim(
                        index,
                        GroundedClaimKindIR::TemporalRelation,
                        format!("temporal event {} is recorded in dialogue", event.event_id),
                        ClaimEpistemicStatusIR::Derived,
                        ClaimSupportStatusIR::DerivedFromDialogueRecords,
                        vec![event.event_id.clone()],
                        vec![event.report_turn],
                    )
                }),
        );
    }
    if claims.is_empty() {
        claims.push(claim(
            0,
            GroundedClaimKindIR::EvidenceAbsence,
            format!("temporal query has disposition {:?}", answer.disposition),
            ClaimEpistemicStatusIR::Unknown,
            ClaimSupportStatusIR::EvidenceAbsent,
            vec![format!("TEMPORAL_QUERY:{:?}", answer.query.kind)],
            vec![turn_index],
        ));
    }
    claims
}

fn dialogue_relation_claims(
    answer: &DialogueRelationAnswerIR,
    turn_index: u64,
) -> Vec<GroundedClaimIR> {
    if answer.evidence.is_empty() {
        return vec![claim(
            0,
            GroundedClaimKindIR::EvidenceAbsence,
            format!(
                "dialogue relation query has disposition {:?}",
                answer.disposition
            ),
            ClaimEpistemicStatusIR::Unknown,
            ClaimSupportStatusIR::EvidenceAbsent,
            vec![format!("DIALOGUE_QUERY:{:?}", answer.query.kind)],
            vec![turn_index],
        )];
    }
    answer
        .evidence
        .iter()
        .enumerate()
        .map(|(index, evidence)| {
            claim(
                index,
                GroundedClaimKindIR::DialogueRelation,
                format!(
                    "{} {:?} {}",
                    evidence.source_summary, evidence.kind, evidence.target_summary
                ),
                ClaimEpistemicStatusIR::Derived,
                ClaimSupportStatusIR::DerivedFromDialogueRecords,
                vec![
                    evidence.relation_id.clone(),
                    evidence.source_belief_id.clone(),
                    evidence.target_belief_id.clone(),
                ],
                vec![evidence.source_turn, evidence.target_turn],
            )
        })
        .collect()
}

fn guard_claims(evaluations: &[ConditionalGuardEvaluationIR]) -> Vec<GroundedClaimIR> {
    evaluations
        .iter()
        .enumerate()
        .map(|(index, evaluation)| {
            let mut evidence = vec![evaluation.guard_id.clone()];
            evidence.extend(
                evaluation
                    .evidence
                    .iter()
                    .map(|item| item.belief_id.clone()),
            );
            let mut turns = vec![evaluation.evaluation_turn];
            turns.extend(evaluation.evidence.iter().map(|item| item.introduced_turn));
            claim(
                index,
                GroundedClaimKindIR::ConditionalGuard,
                format!(
                    "guard {} has status {:?}",
                    evaluation.guard_id, evaluation.status
                ),
                ClaimEpistemicStatusIR::Derived,
                ClaimSupportStatusIR::DerivedFromDialogueRecords,
                evidence,
                turns,
            )
        })
        .collect()
}

fn absence_evidence(ledger: &ActionStateLedgerIR) -> Vec<String> {
    ledger.current_record().map_or_else(
        || vec!["ACTION_LEDGER:NO_RECORDED_RESULT".to_string()],
        |record| vec![record.action_id.clone()],
    )
}

fn claim(
    index: usize,
    kind: GroundedClaimKindIR,
    proposition: String,
    epistemic_status: ClaimEpistemicStatusIR,
    support_status: ClaimSupportStatusIR,
    mut evidence_refs: Vec<String>,
    mut source_turns: Vec<u64>,
) -> GroundedClaimIR {
    evidence_refs.sort();
    evidence_refs.dedup();
    source_turns.retain(|turn| *turn > 0);
    source_turns.sort_unstable();
    source_turns.dedup();
    GroundedClaimIR {
        claim_id: format!("CLAIM-{index:02}"),
        kind,
        proposition,
        epistemic_status,
        support_status,
        evidence_refs,
        source_turns,
        verified: support_status == ClaimSupportStatusIR::VerifiedEvidence,
        semantic_authority: false,
        external_action_executed: false,
    }
}

fn reseal_claim_ids(claims: &mut [GroundedClaimIR]) {
    for (index, claim) in claims.iter_mut().enumerate() {
        let payload = serde_json::to_vec(&(
            index,
            claim.kind,
            &claim.proposition,
            claim.epistemic_status,
            claim.support_status,
            &claim.evidence_refs,
            &claim.source_turns,
        ))
        .expect("claim hash payload");
        let digest = format!("{:x}", Sha256::digest(payload));
        claim.claim_id = format!("CLAIM-{}", &digest[..16]);
    }
}

fn realization_sha256(realization: &EvidenceGroundedRealizationIR) -> String {
    let payload = serde_json::to_vec(&(
        realization.schema.as_str(),
        realization.language,
        realization.realized_text.as_str(),
        &realization.claims,
        realization.unsupported_claims,
        realization.faithful,
        realization.semantic_authority,
        realization.external_action_executed,
    ))
    .expect("realization hash payload");
    format!("{:x}", Sha256::digest(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_presupposition_is_explicit_absence_not_empty_derived_evidence() {
        let answer = DiscourseAnswerIR {
            schema: crate::discourse_qa::DISCOURSE_ANSWER_SCHEMA.to_string(),
            query: crate::discourse_qa::DiscourseQueryIR {
                schema: crate::discourse_qa::DISCOURSE_QUERY_SCHEMA.to_string(),
                original_text: "Why did the queue fail?".to_string(),
                kind: crate::discourse_qa::DiscourseQueryKindIR::PresuppositionCheck,
                requested_source: None,
                requested_attitudes: Vec::new(),
                topic_terms: vec!["queue".to_string(), "fail".to_string()],
                temporal_scope: crate::discourse_qa::QueryTemporalScopeIR::Current,
                presuppositions: vec![crate::discourse_qa::PresuppositionIR {
                    kind: crate::discourse_qa::PresuppositionKindIR::EventOccurred,
                    surface_text: "the queue fail".to_string(),
                    dialogue_truth_established: false,
                }],
                confidence_millis: 900,
            },
            disposition:
                crate::discourse_qa::DiscourseAnswerDispositionIR::PresuppositionUnverified,
            evidence: Vec::new(),
            claims: vec![crate::discourse_qa::DiscourseAnswerClaimIR {
                claim_id: "CLAIM-PRESUPPOSITION".to_string(),
                kind: crate::discourse_qa::AnswerClaimKindIR::PresuppositionNotEstablished,
                subject: "QUERY_PRESUPPOSITION".to_string(),
                value: "the queue fail".to_string(),
                evidence_belief_ids: Vec::new(),
            }],
            language: LanguageCodeIR::English,
            realized_text: "The premise is not established.".to_string(),
            dialogue_truth_established: false,
            external_execution_authorized: false,
            unsupported_claims: 0,
        };
        assert!(answer.validate());
        let ledger = ActionStateLedgerIR::default();
        let analysis = ActionStateAnalysisIR::default();
        let realization = build_evidence_grounded_realization(GroundedRealizationSources {
            language: LanguageCodeIR::English,
            realized_text: &answer.realized_text,
            turn_index: 1,
            plan: None,
            action_analysis: &analysis,
            action_ledger: &ledger,
            competing_outcome_reports: false,
            epistemic_ledger: None,
            discourse_group_update: None,
            topic_transition: None,
            active_topic: None,
            topic_anchored_reference: None,
            discourse_answer: Some(&answer),
            dialogue_relation_answer: None,
            temporal_answer: None,
            guard_evaluations: &[],
            evidence_absence: false,
            source_unsupported_claims: 0,
        });
        assert!(realization.validate());
        assert_eq!(
            realization.claims[0].support_status,
            ClaimSupportStatusIR::EvidenceAbsent
        );
        assert_eq!(realization.claims[0].source_turns, vec![1]);
        assert!(!realization.claims[0].evidence_refs.is_empty());
    }

    #[test]
    fn text_only_interaction_is_nonfactual_and_hash_sealed() {
        let ledger = ActionStateLedgerIR::default();
        let analysis = ActionStateAnalysisIR::default();
        let realization = build_evidence_grounded_realization(GroundedRealizationSources {
            language: LanguageCodeIR::English,
            realized_text: "You're welcome.",
            turn_index: 1,
            plan: None,
            action_analysis: &analysis,
            action_ledger: &ledger,
            competing_outcome_reports: false,
            epistemic_ledger: None,
            discourse_group_update: None,
            topic_transition: None,
            active_topic: None,
            topic_anchored_reference: None,
            discourse_answer: None,
            dialogue_relation_answer: None,
            temporal_answer: None,
            guard_evaluations: &[],
            evidence_absence: false,
            source_unsupported_claims: 0,
        });
        assert!(realization.validate());
        assert_eq!(
            realization.claims[0].kind,
            GroundedClaimKindIR::InteractionState
        );
        assert!(!realization.claims[0].verified);
    }

    #[test]
    fn tampered_realization_hash_is_rejected() {
        let ledger = ActionStateLedgerIR::default();
        let analysis = ActionStateAnalysisIR::default();
        let mut realization = build_evidence_grounded_realization(GroundedRealizationSources {
            language: LanguageCodeIR::Korean,
            realized_text: "알겠어.",
            turn_index: 1,
            plan: None,
            action_analysis: &analysis,
            action_ledger: &ledger,
            competing_outcome_reports: false,
            epistemic_ledger: None,
            discourse_group_update: None,
            topic_transition: None,
            active_topic: None,
            topic_anchored_reference: None,
            discourse_answer: None,
            dialogue_relation_answer: None,
            temporal_answer: None,
            guard_evaluations: &[],
            evidence_absence: false,
            source_unsupported_claims: 0,
        });
        realization.realized_text.push_str(" 완료했어.");
        assert!(!realization.validate());
    }

    #[test]
    fn topic_anchored_claim_cites_resolution_topic_and_membership_hashes() {
        let ledger = ActionStateLedgerIR::default();
        let analysis = ActionStateAnalysisIR::default();
        let mut reference = TopicAnchoredReferenceIR {
            schema: crate::conversation::TOPIC_ANCHORED_REFERENCE_SCHEMA.to_string(),
            applied: true,
            kind: crate::conversation::TopicAnchoredReferentKindIR::ActionMember,
            selector: crate::conversation::TopicAnchoredSelectorKindIR::Ordinal,
            original_text: "inspect the first one".to_string(),
            resolved_text: "inspect cache".to_string(),
            source_surface: "the first one".to_string(),
            topic_id: "TOPIC-R40".to_string(),
            topic_sha256: "a".repeat(64),
            anchor_kind: crate::conversation::DiscourseTopicAnchorKindIR::ActionGroup,
            group_id: "DG-R40".to_string(),
            group_revision: 1,
            membership_sha256: "b".repeat(64),
            member_keys: vec!["GOAL-A".to_string(), "GOAL-B".to_string()],
            selected_member_keys: vec!["GOAL-A".to_string()],
            unresolved_terms: Vec::new(),
            semantic_authority: false,
            external_execution_authorized: false,
            resolution_sha256: String::new(),
        };
        reference.resolution_sha256 =
            crate::conversation::topic_anchored_reference_sha256(&reference);
        assert!(reference.validate());
        let realization = build_evidence_grounded_realization(GroundedRealizationSources {
            language: LanguageCodeIR::English,
            realized_text: "I will inspect cache.",
            turn_index: 3,
            plan: None,
            action_analysis: &analysis,
            action_ledger: &ledger,
            competing_outcome_reports: false,
            epistemic_ledger: None,
            discourse_group_update: None,
            topic_transition: None,
            active_topic: None,
            topic_anchored_reference: Some(&reference),
            discourse_answer: None,
            dialogue_relation_answer: None,
            temporal_answer: None,
            guard_evaluations: &[],
            evidence_absence: false,
            source_unsupported_claims: 0,
        });
        assert!(realization.validate());
        assert_eq!(
            realization.claims[0].kind,
            GroundedClaimKindIR::TopicAnchoredReference
        );
        for expected in [
            &reference.resolution_sha256,
            &reference.topic_sha256,
            &reference.membership_sha256,
        ] {
            assert!(realization.claims[0].evidence_refs.contains(expected));
        }
    }
}
