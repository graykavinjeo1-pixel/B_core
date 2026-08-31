//! Typed, executable causal-mechanism memory.
//!
//! Prose is not capability authority. Only validated mechanisms with explicit
//! prerequisites, effects, observation surface, cost, risk, provenance, and
//! authority can enter this bounded store and later participate in search.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::deliberation::{
    validate_causal_mechanism, CausalMechanismIR, DeliberationEngine, DeliberationError,
    DeliberationIR, DeliberationRequestIR,
};

pub const MECHANISM_KNOWLEDGE_SCHEMA: &str = "B_CORE_MECHANISM_KNOWLEDGE_IR_1";
pub const MECHANISM_MEMORY_SNAPSHOT_SCHEMA: &str = "B_CORE_MECHANISM_MEMORY_SNAPSHOT_IR_1";
pub const KNOWLEDGE_GROUNDED_DELIBERATION_SCHEMA: &str =
    "B_CORE_KNOWLEDGE_GROUNDED_DELIBERATION_IR_1";
pub const DEFAULT_MECHANISM_MEMORY_CAPACITY: usize = 4_096;
const MAX_TAGS: usize = 64;
const MAX_EVIDENCE: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismKnowledgeIR {
    pub schema: String,
    pub knowledge_id: String,
    pub mechanism: CausalMechanismIR,
    pub semantic_tags: Vec<String>,
    pub validation_evidence_refs: Vec<String>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismKnowledgeInjectionReceiptIR {
    pub knowledge_id: String,
    pub content_sha256: String,
    pub inserted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evicted_knowledge_id: Option<String>,
    pub retained_knowledge: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismQueryIR {
    #[serde(default)]
    pub semantic_tags: Vec<String>,
    #[serde(default)]
    pub known_proposition_ids: Vec<String>,
    #[serde(default)]
    pub goal_proposition_ids: Vec<String>,
    pub max_results: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecalledMechanismIR {
    pub knowledge: MechanismKnowledgeIR,
    pub relevance_score: i32,
    pub matched_tags: Vec<String>,
    pub matched_known_propositions: Vec<String>,
    pub matched_goal_propositions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismMemorySnapshotIR {
    pub schema: String,
    pub knowledge: Vec<MechanismKnowledgeIR>,
    pub knowledge_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeGroundedDeliberationIR {
    pub schema: String,
    pub request_id: String,
    pub recalled_mechanisms: Vec<RecalledMechanismIR>,
    pub deliberation: DeliberationIR,
    pub result_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MechanismMemoryError {
    InvalidSchema,
    InvalidKnowledge,
    InvalidQuery,
    InvalidSnapshot,
    IdentityConflict,
    MechanismConflict,
    Deliberation,
}

#[derive(Debug)]
pub struct MechanismMemory {
    capacity: usize,
    records: BTreeMap<String, (String, MechanismKnowledgeIR)>,
    insertion_order: VecDeque<String>,
}

impl Default for MechanismMemory {
    fn default() -> Self {
        Self::new(DEFAULT_MECHANISM_MEMORY_CAPACITY)
    }
}

impl MechanismMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            records: BTreeMap::new(),
            insertion_order: VecDeque::new(),
        }
    }

    pub fn inject(
        &mut self,
        knowledge: MechanismKnowledgeIR,
    ) -> Result<MechanismKnowledgeInjectionReceiptIR, MechanismMemoryError> {
        validate_knowledge(&knowledge)?;
        let content_sha256 = sha256_json(&knowledge);
        if let Some((existing, _)) = self.records.get(&knowledge.knowledge_id) {
            if existing != &content_sha256 {
                return Err(MechanismMemoryError::IdentityConflict);
            }
            return Ok(MechanismKnowledgeInjectionReceiptIR {
                knowledge_id: knowledge.knowledge_id,
                content_sha256,
                inserted: false,
                evicted_knowledge_id: None,
                retained_knowledge: self.records.len(),
            });
        }
        if self.records.values().any(|(_, existing)| {
            existing.mechanism.mechanism_id == knowledge.mechanism.mechanism_id
                && existing.mechanism != knowledge.mechanism
        }) {
            return Err(MechanismMemoryError::MechanismConflict);
        }
        let evicted_knowledge_id = if self.records.len() == self.capacity {
            self.insertion_order.pop_front().inspect(|oldest| {
                self.records.remove(oldest);
            })
        } else {
            None
        };
        self.insertion_order
            .push_back(knowledge.knowledge_id.clone());
        self.records.insert(
            knowledge.knowledge_id.clone(),
            (content_sha256.clone(), knowledge.clone()),
        );
        Ok(MechanismKnowledgeInjectionReceiptIR {
            knowledge_id: knowledge.knowledge_id,
            content_sha256,
            inserted: true,
            evicted_knowledge_id,
            retained_knowledge: self.records.len(),
        })
    }

    pub fn recall(
        &self,
        query: &MechanismQueryIR,
    ) -> Result<Vec<RecalledMechanismIR>, MechanismMemoryError> {
        validate_query(query)?;
        let query_tags = normalized_set(&query.semantic_tags);
        let known = normalized_set(&query.known_proposition_ids);
        let goals = normalized_set(&query.goal_proposition_ids);
        let mut recalled = self
            .records
            .values()
            .filter_map(|(_, knowledge)| {
                let tags = normalized_set(&knowledge.semantic_tags);
                let prerequisites = knowledge
                    .mechanism
                    .prerequisites
                    .iter()
                    .map(|literal| literal.proposition_id.to_ascii_lowercase())
                    .collect::<BTreeSet<_>>();
                let effects = knowledge
                    .mechanism
                    .effects
                    .iter()
                    .map(|literal| literal.proposition_id.to_ascii_lowercase())
                    .collect::<BTreeSet<_>>();
                let observed = knowledge
                    .mechanism
                    .observes
                    .iter()
                    .map(|value| value.to_ascii_lowercase())
                    .collect::<BTreeSet<_>>();
                let matched_tags = query_tags.intersection(&tags).cloned().collect::<Vec<_>>();
                let matched_known_propositions = known
                    .intersection(&prerequisites)
                    .cloned()
                    .collect::<Vec<_>>();
                let matched_goal_propositions = goals
                    .iter()
                    .filter(|goal| effects.contains(*goal) || observed.contains(*goal))
                    .cloned()
                    .collect::<Vec<_>>();
                let score = i32::try_from(matched_goal_propositions.len())
                    .unwrap_or(i32::MAX)
                    .saturating_mul(500)
                    .saturating_add(
                        i32::try_from(matched_known_propositions.len())
                            .unwrap_or(i32::MAX)
                            .saturating_mul(250),
                    )
                    .saturating_add(
                        i32::try_from(matched_tags.len())
                            .unwrap_or(i32::MAX)
                            .saturating_mul(100),
                    )
                    .saturating_add(i32::from(knowledge.confidence_millis) / 20);
                (score > 0).then(|| RecalledMechanismIR {
                    knowledge: knowledge.clone(),
                    relevance_score: score,
                    matched_tags,
                    matched_known_propositions,
                    matched_goal_propositions,
                })
            })
            .collect::<Vec<_>>();
        recalled.sort_by(|left, right| {
            right
                .relevance_score
                .cmp(&left.relevance_score)
                .then_with(|| {
                    left.knowledge
                        .knowledge_id
                        .cmp(&right.knowledge.knowledge_id)
                })
        });
        recalled.truncate(query.max_results);
        Ok(recalled)
    }

    pub fn deliberate(
        &self,
        request: &DeliberationRequestIR,
        query: &MechanismQueryIR,
    ) -> Result<KnowledgeGroundedDeliberationIR, MechanismMemoryError> {
        let recalled_mechanisms = self.recall(query)?;
        let mut enriched = request.clone();
        let mut identities = enriched
            .mechanisms
            .iter()
            .map(|mechanism| (mechanism.mechanism_id.clone(), mechanism.clone()))
            .collect::<BTreeMap<_, _>>();
        for recalled in &recalled_mechanisms {
            let mechanism = &recalled.knowledge.mechanism;
            if let Some(existing) = identities.get(&mechanism.mechanism_id) {
                if existing != mechanism {
                    return Err(MechanismMemoryError::MechanismConflict);
                }
                continue;
            }
            identities.insert(mechanism.mechanism_id.clone(), mechanism.clone());
            enriched.mechanisms.push(mechanism.clone());
        }
        let deliberation = DeliberationEngine
            .deliberate(&enriched)
            .map_err(map_deliberation_error)?;
        let mut result = KnowledgeGroundedDeliberationIR {
            schema: KNOWLEDGE_GROUNDED_DELIBERATION_SCHEMA.to_string(),
            request_id: request.request_id.clone(),
            recalled_mechanisms,
            deliberation,
            result_sha256: String::new(),
        };
        result.result_sha256 = grounded_result_hash(&result);
        Ok(result)
    }

    pub fn snapshot(&self) -> MechanismMemorySnapshotIR {
        let knowledge = self
            .insertion_order
            .iter()
            .filter_map(|id| self.records.get(id).map(|(_, item)| item.clone()))
            .collect::<Vec<_>>();
        MechanismMemorySnapshotIR {
            schema: MECHANISM_MEMORY_SNAPSHOT_SCHEMA.to_string(),
            knowledge_sha256: sha256_json(&knowledge),
            knowledge,
        }
    }

    pub fn import_snapshot(
        &mut self,
        snapshot: &MechanismMemorySnapshotIR,
    ) -> Result<Vec<MechanismKnowledgeInjectionReceiptIR>, MechanismMemoryError> {
        if snapshot.schema != MECHANISM_MEMORY_SNAPSHOT_SCHEMA
            || snapshot.knowledge.len() > self.capacity
            || snapshot.knowledge_sha256 != sha256_json(&snapshot.knowledge)
        {
            return Err(MechanismMemoryError::InvalidSnapshot);
        }
        let mut ids = BTreeSet::new();
        let mut mechanism_ids = BTreeSet::new();
        for item in &snapshot.knowledge {
            validate_knowledge(item)?;
            if !ids.insert(item.knowledge_id.clone())
                || !mechanism_ids.insert(item.mechanism.mechanism_id.clone())
            {
                return Err(MechanismMemoryError::InvalidSnapshot);
            }
        }
        snapshot
            .knowledge
            .iter()
            .cloned()
            .map(|item| self.inject(item))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

fn validate_knowledge(knowledge: &MechanismKnowledgeIR) -> Result<(), MechanismMemoryError> {
    if knowledge.schema != MECHANISM_KNOWLEDGE_SCHEMA
        || knowledge.knowledge_id.trim().is_empty()
        || knowledge.knowledge_id.len() > 128
        || knowledge.semantic_tags.is_empty()
        || knowledge.semantic_tags.len() > MAX_TAGS
        || knowledge
            .semantic_tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 128)
        || knowledge.validation_evidence_refs.is_empty()
        || knowledge.validation_evidence_refs.len() > MAX_EVIDENCE
        || knowledge
            .validation_evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty() || reference.len() > 1_024)
        || knowledge.confidence_millis == 0
        || knowledge.confidence_millis > 1_000
        || validate_causal_mechanism(&knowledge.mechanism).is_err()
    {
        return Err(MechanismMemoryError::InvalidKnowledge);
    }
    Ok(())
}

fn validate_query(query: &MechanismQueryIR) -> Result<(), MechanismMemoryError> {
    if query.max_results == 0
        || query.max_results > 128
        || (query.semantic_tags.is_empty()
            && query.known_proposition_ids.is_empty()
            && query.goal_proposition_ids.is_empty())
        || query.semantic_tags.len() > MAX_TAGS
        || query.known_proposition_ids.len() > MAX_TAGS
        || query.goal_proposition_ids.len() > MAX_TAGS
        || query
            .semantic_tags
            .iter()
            .chain(&query.known_proposition_ids)
            .chain(&query.goal_proposition_ids)
            .any(|value| value.trim().is_empty() || value.len() > 128)
    {
        return Err(MechanismMemoryError::InvalidQuery);
    }
    Ok(())
}

fn map_deliberation_error(_: DeliberationError) -> MechanismMemoryError {
    MechanismMemoryError::Deliberation
}

fn grounded_result_hash(result: &KnowledgeGroundedDeliberationIR) -> String {
    let mut unsigned = result.clone();
    unsigned.result_sha256.clear();
    sha256_json(&unsigned)
}

fn normalized_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
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
    use crate::deliberation::{
        ActionAuthorityIR, AuthorityEnvelopeIR, DeliberationDispositionIR, EvidenceIR, LiteralIR,
        MechanismKindIR, DELIBERATION_REQUEST_SCHEMA,
    };

    fn literal(id: &str) -> LiteralIR {
        LiteralIR {
            proposition_id: id.to_string(),
            value: true,
        }
    }

    fn knowledge(
        knowledge_id: &str,
        mechanism_id: &str,
        prerequisite: &str,
        effect: &str,
    ) -> MechanismKnowledgeIR {
        MechanismKnowledgeIR {
            schema: MECHANISM_KNOWLEDGE_SCHEMA.to_string(),
            knowledge_id: knowledge_id.to_string(),
            mechanism: CausalMechanismIR {
                mechanism_id: mechanism_id.to_string(),
                kind: MechanismKindIR::Inference,
                prerequisites: vec![literal(prerequisite)],
                effects: vec![literal(effect)],
                observes: Vec::new(),
                authority: ActionAuthorityIR::InternalInference,
                authorized: true,
                reversible: true,
                recovery_reference: None,
                cost_millis: 10,
                risk_millis: 0,
                provenance_refs: vec![format!("test:{knowledge_id}")],
            },
            semantic_tags: vec!["repair".to_string(), "causal".to_string()],
            validation_evidence_refs: vec![format!("test:{knowledge_id}:pass")],
            confidence_millis: 950,
        }
    }

    #[test]
    fn recalled_typed_mechanisms_compose_into_a_new_solution() {
        let mut memory = MechanismMemory::new(8);
        memory
            .inject(knowledge("K-LOCALIZE", "LOCALIZE", "FAILURE", "CAUSE"))
            .unwrap();
        memory
            .inject(knowledge("K-SOLVE", "SOLVE", "CAUSE", "REPAIRED"))
            .unwrap();
        let result = memory
            .deliberate(
                &DeliberationRequestIR {
                    schema: DELIBERATION_REQUEST_SCHEMA.to_string(),
                    request_id: "MEMORY-THINK-1".to_string(),
                    subject: "compose stored mechanisms".to_string(),
                    evidence: vec![EvidenceIR {
                        evidence_id: "E-FAILURE".to_string(),
                        literal: literal("FAILURE"),
                        reliability_millis: 990,
                        source_ref: "test:failure".to_string(),
                    }],
                    mechanisms: Vec::new(),
                    goals: vec![literal("REPAIRED")],
                    authority_envelope: AuthorityEnvelopeIR::default(),
                    immutable_constraints: Vec::new(),
                    max_depth: 4,
                    beam_width: 8,
                    max_hypotheses: 8,
                    max_counterfactuals: 8,
                },
                &MechanismQueryIR {
                    semantic_tags: vec!["repair".to_string()],
                    known_proposition_ids: vec!["FAILURE".to_string(), "CAUSE".to_string()],
                    goal_proposition_ids: vec!["REPAIRED".to_string()],
                    max_results: 8,
                },
            )
            .unwrap();
        assert_eq!(result.recalled_mechanisms.len(), 2);
        assert_eq!(
            result.deliberation.disposition,
            DeliberationDispositionIR::GoalReachable
        );
        assert_eq!(
            result.deliberation.selected_plan.unwrap().mechanism_ids,
            ["LOCALIZE", "SOLVE"]
        );
    }

    #[test]
    fn snapshot_is_tamper_evident_and_identity_safe() {
        let mut source = MechanismMemory::new(4);
        source.inject(knowledge("K-1", "M-1", "A", "B")).unwrap();
        let snapshot = source.snapshot();
        let mut destination = MechanismMemory::new(4);
        assert_eq!(destination.import_snapshot(&snapshot).unwrap().len(), 1);
        let mut tampered = snapshot;
        tampered.knowledge[0].confidence_millis = 1;
        assert_eq!(
            destination.import_snapshot(&tampered),
            Err(MechanismMemoryError::InvalidSnapshot)
        );
    }
}
