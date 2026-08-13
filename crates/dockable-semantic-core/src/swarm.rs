use std::{collections::BTreeSet, thread};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SWARM_DELIBERATION_REQUEST_SCHEMA: &str = "B_CORE_SWARM_DELIBERATION_REQUEST_IR_1";
pub const SWARM_DELIBERATION_SCHEMA: &str = "B_CORE_SWARM_DELIBERATION_IR_1";
const MAX_FACTS: usize = 64;
const MAX_WORKERS: usize = 6;
const MAX_ROUNDS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QualityCriterionIR {
    RequirementCoverage,
    EvidenceIntegrity,
    StructureIntegrity,
    AudienceUsability,
    QuantitativeIntegrity,
    ContradictionResistance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpertWorkerRoleIR {
    RequirementAnalyst,
    EvidenceAuditor,
    StructureEditor,
    AudienceAdvocate,
    QuantitativeAuditor,
    AdversarialCritic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssessmentVerdictIR {
    Pass,
    Warning,
    Fail,
}

impl AssessmentVerdictIR {
    fn severity(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::Warning => 1,
            Self::Fail => 2,
        }
    }

    fn worst(left: Self, right: Self) -> Self {
        if left.severity() >= right.severity() {
            left
        } else {
            right
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationFactIR {
    pub fact_id: String,
    pub criterion: QualityCriterionIR,
    pub verdict: AssessmentVerdictIR,
    /// Stable machine-readable reason; free-form expert prose is not authority.
    pub rationale_code: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDeliberationRequestIR {
    pub schema: String,
    pub request_id: String,
    pub subject: String,
    pub parent_reasoning_sha256: String,
    pub facts: Vec<DeliberationFactIR>,
    pub max_workers: usize,
    pub max_rounds: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpertWorkerIR {
    pub worker_id: String,
    pub role: ExpertWorkerRoleIR,
    pub spawn_reason: String,
    pub assigned_fact_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpertContributionIR {
    pub contribution_id: String,
    pub worker_id: String,
    pub criterion: QualityCriterionIR,
    pub verdict: AssessmentVerdictIR,
    pub assessed_fact_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub reasoning_trace_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PeerReviewDispositionIR {
    Endorse,
    Challenge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerMessageIR {
    pub message_id: String,
    pub from_worker_id: String,
    pub to_worker_id: String,
    pub target_contribution_id: String,
    pub disposition: PeerReviewDispositionIR,
    pub reason_code: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriterionDecisionIR {
    pub criterion: QualityCriterionIR,
    pub verdict: AssessmentVerdictIR,
    pub contribution_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmDeliberationIR {
    pub schema: String,
    pub request_id: String,
    pub subject: String,
    pub parent_reasoning_sha256: String,
    pub workers: Vec<ExpertWorkerIR>,
    pub contributions: Vec<ExpertContributionIR>,
    pub peer_messages: Vec<PeerMessageIR>,
    pub decisions: Vec<CriterionDecisionIR>,
    pub accepted: bool,
    pub rejection_codes: Vec<String>,
    pub rounds_completed: usize,
    pub parallel_lanes: usize,
    pub worker_spawn_count: usize,
    pub external_model_calls: usize,
    pub deliberation_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwarmError {
    InvalidSchema,
    InvalidRequest,
    ResourceBoundInsufficient,
    WorkerPanic,
}

#[derive(Debug, Default)]
pub struct SwarmCore;

impl SwarmCore {
    pub fn deliberate(
        &self,
        request: &SwarmDeliberationRequestIR,
    ) -> Result<SwarmDeliberationIR, SwarmError> {
        validate_request(request)?;
        let workers = spawn_needed_workers(request)?;
        let contributions = run_workers_in_parallel(&workers, &request.facts)?;
        let peer_messages = peer_review(&workers, &contributions, &request.facts);
        let decisions = contributions
            .iter()
            .map(|contribution| CriterionDecisionIR {
                criterion: contribution.criterion,
                verdict: contribution.verdict,
                contribution_id: contribution.contribution_id.clone(),
            })
            .collect::<Vec<_>>();
        let mut rejection_codes = Vec::new();
        for decision in &decisions {
            if decision.verdict == AssessmentVerdictIR::Fail {
                rejection_codes.push(format!("{:?}_FAILED", decision.criterion).to_uppercase());
            }
        }
        if peer_messages.iter().any(|message| {
            message.disposition == PeerReviewDispositionIR::Challenge && !message.resolved
        }) {
            rejection_codes.push("UNRESOLVED_PEER_CHALLENGE".to_string());
        }
        let covered = contributions
            .iter()
            .flat_map(|contribution| contribution.assessed_fact_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        if covered.len() != request.facts.len() {
            rejection_codes.push("INCOMPLETE_FACT_COVERAGE".to_string());
        }
        rejection_codes.sort();
        rejection_codes.dedup();
        let accepted = rejection_codes.is_empty();
        let rounds_completed = if workers.len() > 1 {
            request.max_rounds.min(2)
        } else {
            1
        };
        let mut result = SwarmDeliberationIR {
            schema: SWARM_DELIBERATION_SCHEMA.to_string(),
            request_id: request.request_id.clone(),
            subject: request.subject.clone(),
            parent_reasoning_sha256: request.parent_reasoning_sha256.clone(),
            workers,
            contributions,
            peer_messages,
            decisions,
            accepted,
            rejection_codes,
            rounds_completed,
            parallel_lanes: 0,
            worker_spawn_count: 0,
            external_model_calls: 0,
            deliberation_sha256: String::new(),
        };
        result.parallel_lanes = result.workers.len();
        result.worker_spawn_count = result.workers.len();
        result.deliberation_sha256 = sha256_json(&(
            &result.schema,
            &result.request_id,
            &result.subject,
            &result.parent_reasoning_sha256,
            &result.workers,
            &result.contributions,
            &result.peer_messages,
            &result.decisions,
            result.accepted,
            &result.rejection_codes,
            result.rounds_completed,
            result.parallel_lanes,
            result.worker_spawn_count,
            result.external_model_calls,
        ));
        Ok(result)
    }
}

fn validate_request(request: &SwarmDeliberationRequestIR) -> Result<(), SwarmError> {
    if request.schema != SWARM_DELIBERATION_REQUEST_SCHEMA {
        return Err(SwarmError::InvalidSchema);
    }
    if request.request_id.trim().is_empty()
        || request.subject.trim().is_empty()
        || request.parent_reasoning_sha256.trim().is_empty()
        || request.facts.is_empty()
        || request.facts.len() > MAX_FACTS
        || !(1..=MAX_WORKERS).contains(&request.max_workers)
        || !(1..=MAX_ROUNDS).contains(&request.max_rounds)
    {
        return Err(SwarmError::InvalidRequest);
    }
    let mut ids = BTreeSet::new();
    for fact in &request.facts {
        if fact.fact_id.trim().is_empty()
            || fact.rationale_code.trim().is_empty()
            || fact.evidence_refs.is_empty()
            || fact
                .evidence_refs
                .iter()
                .any(|reference| reference.trim().is_empty())
            || !ids.insert(fact.fact_id.clone())
        {
            return Err(SwarmError::InvalidRequest);
        }
    }
    Ok(())
}

fn role_for(criterion: QualityCriterionIR) -> ExpertWorkerRoleIR {
    match criterion {
        QualityCriterionIR::RequirementCoverage => ExpertWorkerRoleIR::RequirementAnalyst,
        QualityCriterionIR::EvidenceIntegrity => ExpertWorkerRoleIR::EvidenceAuditor,
        QualityCriterionIR::StructureIntegrity => ExpertWorkerRoleIR::StructureEditor,
        QualityCriterionIR::AudienceUsability => ExpertWorkerRoleIR::AudienceAdvocate,
        QualityCriterionIR::QuantitativeIntegrity => ExpertWorkerRoleIR::QuantitativeAuditor,
        QualityCriterionIR::ContradictionResistance => ExpertWorkerRoleIR::AdversarialCritic,
    }
}

fn spawn_needed_workers(
    request: &SwarmDeliberationRequestIR,
) -> Result<Vec<ExpertWorkerIR>, SwarmError> {
    let criteria = request
        .facts
        .iter()
        .map(|fact| fact.criterion)
        .collect::<BTreeSet<_>>();
    if criteria.len() > request.max_workers {
        return Err(SwarmError::ResourceBoundInsufficient);
    }
    Ok(criteria
        .into_iter()
        .enumerate()
        .map(|(index, criterion)| {
            let role = role_for(criterion);
            ExpertWorkerIR {
                worker_id: format!("WORKER-{:02}", index + 1),
                role,
                spawn_reason: format!("REQUIRED_{criterion:?}").to_uppercase(),
                assigned_fact_ids: request
                    .facts
                    .iter()
                    .filter(|fact| fact.criterion == criterion)
                    .map(|fact| fact.fact_id.clone())
                    .collect(),
            }
        })
        .collect())
}

fn criterion_for(role: ExpertWorkerRoleIR) -> QualityCriterionIR {
    match role {
        ExpertWorkerRoleIR::RequirementAnalyst => QualityCriterionIR::RequirementCoverage,
        ExpertWorkerRoleIR::EvidenceAuditor => QualityCriterionIR::EvidenceIntegrity,
        ExpertWorkerRoleIR::StructureEditor => QualityCriterionIR::StructureIntegrity,
        ExpertWorkerRoleIR::AudienceAdvocate => QualityCriterionIR::AudienceUsability,
        ExpertWorkerRoleIR::QuantitativeAuditor => QualityCriterionIR::QuantitativeIntegrity,
        ExpertWorkerRoleIR::AdversarialCritic => QualityCriterionIR::ContradictionResistance,
    }
}

fn run_workers_in_parallel(
    workers: &[ExpertWorkerIR],
    facts: &[DeliberationFactIR],
) -> Result<Vec<ExpertContributionIR>, SwarmError> {
    let mut contributions = thread::scope(|scope| {
        let mut handles = Vec::new();
        for worker in workers.iter().cloned() {
            handles.push(scope.spawn(move || evaluate_worker(worker, facts)));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().map_err(|_| SwarmError::WorkerPanic))
            .collect::<Result<Vec<_>, _>>()
    })?;
    contributions.sort_by(|left, right| left.contribution_id.cmp(&right.contribution_id));
    Ok(contributions)
}

fn evaluate_worker(worker: ExpertWorkerIR, facts: &[DeliberationFactIR]) -> ExpertContributionIR {
    let criterion = criterion_for(worker.role);
    let assigned = facts
        .iter()
        .filter(|fact| fact.criterion == criterion)
        .collect::<Vec<_>>();
    let verdict = assigned
        .iter()
        .fold(AssessmentVerdictIR::Pass, |worst, fact| {
            AssessmentVerdictIR::worst(worst, fact.verdict)
        });
    let mut evidence_refs = assigned
        .iter()
        .flat_map(|fact| fact.evidence_refs.iter().cloned())
        .collect::<Vec<_>>();
    evidence_refs.sort();
    evidence_refs.dedup();
    let assessed_fact_ids = assigned
        .iter()
        .map(|fact| fact.fact_id.clone())
        .collect::<Vec<_>>();
    let reasoning_trace_sha256 = sha256_json(&(
        &worker.worker_id,
        worker.role,
        criterion,
        verdict,
        &assessed_fact_ids,
        &evidence_refs,
    ));
    ExpertContributionIR {
        contribution_id: worker.worker_id.replace("WORKER", "CONTRIBUTION"),
        worker_id: worker.worker_id,
        criterion,
        verdict,
        assessed_fact_ids,
        evidence_refs,
        reasoning_trace_sha256,
    }
}

fn peer_review(
    workers: &[ExpertWorkerIR],
    contributions: &[ExpertContributionIR],
    facts: &[DeliberationFactIR],
) -> Vec<PeerMessageIR> {
    if workers.len() < 2 {
        return Vec::new();
    }
    contributions
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let reviewer = &workers[(index + 1) % workers.len()];
            let expected = facts
                .iter()
                .filter(|fact| fact.criterion == target.criterion)
                .fold(AssessmentVerdictIR::Pass, |worst, fact| {
                    AssessmentVerdictIR::worst(worst, fact.verdict)
                });
            let complete = !target.assessed_fact_ids.is_empty()
                && target
                    .evidence_refs
                    .iter()
                    .all(|reference| !reference.trim().is_empty())
                && expected == target.verdict;
            PeerMessageIR {
                message_id: format!("PEER-{:02}", index + 1),
                from_worker_id: reviewer.worker_id.clone(),
                to_worker_id: target.worker_id.clone(),
                target_contribution_id: target.contribution_id.clone(),
                disposition: if complete {
                    PeerReviewDispositionIR::Endorse
                } else {
                    PeerReviewDispositionIR::Challenge
                },
                reason_code: if complete {
                    "INDEPENDENT_FACT_AND_EVIDENCE_CHECK_PASSED"
                } else {
                    "FACT_COVERAGE_OR_EVIDENCE_MISMATCH"
                }
                .to_string(),
                resolved: complete,
            }
        })
        .collect()
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).unwrap_or_default())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(
        id: &str,
        criterion: QualityCriterionIR,
        verdict: AssessmentVerdictIR,
    ) -> DeliberationFactIR {
        DeliberationFactIR {
            fact_id: id.to_string(),
            criterion,
            verdict,
            rationale_code: format!("{id}_RATIONALE"),
            evidence_refs: vec![format!("document:{id}")],
        }
    }

    fn request(facts: Vec<DeliberationFactIR>) -> SwarmDeliberationRequestIR {
        SwarmDeliberationRequestIR {
            schema: SWARM_DELIBERATION_REQUEST_SCHEMA.to_string(),
            request_id: "SWARM-TEST".to_string(),
            subject: "bounded document review".to_string(),
            parent_reasoning_sha256: "plan-hash".to_string(),
            facts,
            max_workers: 6,
            max_rounds: 2,
        }
    }

    #[test]
    fn spawns_only_workers_required_by_observed_criteria() {
        let result = SwarmCore
            .deliberate(&request(vec![
                fact(
                    "REQ",
                    QualityCriterionIR::RequirementCoverage,
                    AssessmentVerdictIR::Pass,
                ),
                fact(
                    "STRUCT",
                    QualityCriterionIR::StructureIntegrity,
                    AssessmentVerdictIR::Pass,
                ),
            ]))
            .expect("deliberation");
        assert_eq!(result.worker_spawn_count, 2);
        assert!(!result
            .workers
            .iter()
            .any(|worker| worker.role == ExpertWorkerRoleIR::QuantitativeAuditor));
        assert!(result.accepted);
        assert_eq!(result.external_model_calls, 0);
    }

    #[test]
    fn failing_fact_blocks_acceptance() {
        let result = SwarmCore
            .deliberate(&request(vec![fact(
                "MISSING",
                QualityCriterionIR::EvidenceIntegrity,
                AssessmentVerdictIR::Fail,
            )]))
            .expect("deliberation");
        assert!(!result.accepted);
        assert!(!result.rejection_codes.is_empty());
    }

    #[test]
    fn parallel_deliberation_is_deterministic_and_cross_reviewed() {
        let request = request(vec![
            fact(
                "REQ",
                QualityCriterionIR::RequirementCoverage,
                AssessmentVerdictIR::Pass,
            ),
            fact(
                "EVIDENCE",
                QualityCriterionIR::EvidenceIntegrity,
                AssessmentVerdictIR::Warning,
            ),
            fact(
                "CRITIC",
                QualityCriterionIR::ContradictionResistance,
                AssessmentVerdictIR::Pass,
            ),
        ]);
        let first = SwarmCore.deliberate(&request).expect("first");
        let second = SwarmCore.deliberate(&request).expect("second");
        assert_eq!(first.deliberation_sha256, second.deliberation_sha256);
        assert_eq!(first.parallel_lanes, 3);
        assert_eq!(first.peer_messages.len(), 3);
        assert!(first.peer_messages.iter().all(|message| message.resolved));
    }

    #[test]
    fn worker_budget_is_fail_closed() {
        let mut request = request(vec![
            fact(
                "REQ",
                QualityCriterionIR::RequirementCoverage,
                AssessmentVerdictIR::Pass,
            ),
            fact(
                "STRUCT",
                QualityCriterionIR::StructureIntegrity,
                AssessmentVerdictIR::Pass,
            ),
        ]);
        request.max_workers = 1;
        assert_eq!(
            SwarmCore.deliberate(&request),
            Err(SwarmError::ResourceBoundInsufficient)
        );
    }
}
