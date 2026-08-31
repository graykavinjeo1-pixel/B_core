use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const EXPERIENCE_SCHEMA: &str = "B_CORE_EXPERIENCE_IR_1";
pub const EXPERIENCE_SNAPSHOT_SCHEMA: &str = "B_CORE_EXPERIENCE_SNAPSHOT_IR_1";
pub const DEFAULT_EXPERIENCE_CAPACITY: usize = 1_024;
const MAX_EXPERIENCE_TEXT_BYTES: usize = 16 * 1024;
const MAX_TAGS: usize = 64;
const MAX_EVIDENCE_ITEMS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExperienceOutcomeIR {
    Successful,
    Failed,
    Partial,
    Observed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceIR {
    pub schema: String,
    pub experience_id: String,
    pub situation: String,
    pub action: String,
    pub outcome: ExperienceOutcomeIR,
    pub outcome_description: String,
    pub semantic_tags: Vec<String>,
    pub evidence: Vec<String>,
    pub confidence_millis: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceInjectionReceiptIR {
    pub experience_id: String,
    pub content_sha256: String,
    pub inserted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evicted_experience_id: Option<String>,
    pub retained_experiences: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceQueryIR {
    pub semantic_tags: Vec<String>,
    pub text_terms: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_outcome: Option<ExperienceOutcomeIR>,
    pub max_results: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecalledExperienceIR {
    pub experience: ExperienceIR,
    pub relevance_score: i32,
    pub matched_tags: Vec<String>,
    pub matched_text_terms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceSnapshotIR {
    pub schema: String,
    pub experiences: Vec<ExperienceIR>,
    pub experiences_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExperienceError {
    InvalidSchema,
    InvalidIdentity,
    InvalidText,
    InvalidConfidence,
    TooManyTags,
    TooManyEvidenceItems,
    IdentityConflict,
    InvalidQuery,
    InvalidSnapshot,
}

#[derive(Debug)]
pub struct ExperienceMemory {
    capacity: usize,
    records: BTreeMap<String, (String, ExperienceIR)>,
    insertion_order: VecDeque<String>,
}

impl Default for ExperienceMemory {
    fn default() -> Self {
        Self::new(DEFAULT_EXPERIENCE_CAPACITY)
    }
}

impl ExperienceMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            records: BTreeMap::new(),
            insertion_order: VecDeque::new(),
        }
    }

    pub fn inject(
        &mut self,
        experience: ExperienceIR,
    ) -> Result<ExperienceInjectionReceiptIR, ExperienceError> {
        validate_experience(&experience)?;
        let content_sha256 = experience_sha256(&experience);
        if let Some((existing_sha256, _)) = self.records.get(&experience.experience_id) {
            if existing_sha256 != &content_sha256 {
                return Err(ExperienceError::IdentityConflict);
            }
            return Ok(ExperienceInjectionReceiptIR {
                experience_id: experience.experience_id,
                content_sha256,
                inserted: false,
                evicted_experience_id: None,
                retained_experiences: self.records.len(),
            });
        }

        let mut evicted_experience_id = None;
        if self.records.len() == self.capacity {
            if let Some(oldest) = self.insertion_order.pop_front() {
                self.records.remove(&oldest);
                evicted_experience_id = Some(oldest);
            }
        }
        self.insertion_order
            .push_back(experience.experience_id.clone());
        self.records.insert(
            experience.experience_id.clone(),
            (content_sha256.clone(), experience.clone()),
        );
        Ok(ExperienceInjectionReceiptIR {
            experience_id: experience.experience_id,
            content_sha256,
            inserted: true,
            evicted_experience_id,
            retained_experiences: self.records.len(),
        })
    }

    pub fn recall(
        &self,
        query: &ExperienceQueryIR,
    ) -> Result<Vec<RecalledExperienceIR>, ExperienceError> {
        if query.max_results == 0
            || query.max_results > 64
            || (query.semantic_tags.is_empty() && query.text_terms.is_empty())
            || query.semantic_tags.len() > MAX_TAGS
            || query.text_terms.len() > MAX_TAGS
            || query
                .semantic_tags
                .iter()
                .chain(&query.text_terms)
                .any(|value| value.trim().is_empty() || value.len() > 256)
        {
            return Err(ExperienceError::InvalidQuery);
        }
        let query_tags = normalized_set(&query.semantic_tags);
        let query_terms = normalized_set(&query.text_terms);
        let mut recalled = self
            .records
            .values()
            .filter_map(|(_, experience)| {
                let experience_tags = normalized_set(&experience.semantic_tags);
                let matched_tags = query_tags
                    .intersection(&experience_tags)
                    .cloned()
                    .collect::<Vec<_>>();
                let haystack = format!(
                    "{} {} {}",
                    experience.situation, experience.action, experience.outcome_description
                )
                .to_lowercase();
                let matched_text_terms = query_terms
                    .iter()
                    .filter(|term| haystack.contains(term.as_str()))
                    .cloned()
                    .collect::<Vec<_>>();
                let outcome_bonus = if query.preferred_outcome == Some(experience.outcome) {
                    25
                } else {
                    0
                };
                let relevance_score = i32::try_from(matched_tags.len())
                    .unwrap_or(i32::MAX)
                    .saturating_mul(100)
                    .saturating_add(
                        i32::try_from(matched_text_terms.len())
                            .unwrap_or(i32::MAX)
                            .saturating_mul(20),
                    )
                    .saturating_add(outcome_bonus)
                    .saturating_add(i32::from(experience.confidence_millis) / 100);
                (!matched_tags.is_empty() || !matched_text_terms.is_empty()).then(|| {
                    RecalledExperienceIR {
                        experience: experience.clone(),
                        relevance_score,
                        matched_tags,
                        matched_text_terms,
                    }
                })
            })
            .collect::<Vec<_>>();
        recalled.sort_by(|left, right| {
            right
                .relevance_score
                .cmp(&left.relevance_score)
                .then_with(|| {
                    left.experience
                        .experience_id
                        .cmp(&right.experience.experience_id)
                })
        });
        recalled.truncate(query.max_results);
        Ok(recalled)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn export_snapshot(&self) -> ExperienceSnapshotIR {
        let experiences = self
            .insertion_order
            .iter()
            .filter_map(|id| {
                self.records
                    .get(id)
                    .map(|(_, experience)| experience.clone())
            })
            .collect::<Vec<_>>();
        ExperienceSnapshotIR {
            schema: EXPERIENCE_SNAPSHOT_SCHEMA.to_string(),
            experiences_sha256: experience_sequence_sha256(&experiences),
            experiences,
        }
    }

    pub fn import_snapshot(
        &mut self,
        snapshot: &ExperienceSnapshotIR,
    ) -> Result<Vec<ExperienceInjectionReceiptIR>, ExperienceError> {
        if snapshot.schema != EXPERIENCE_SNAPSHOT_SCHEMA
            || snapshot.experiences.len() > self.capacity
            || snapshot.experiences_sha256 != experience_sequence_sha256(&snapshot.experiences)
        {
            return Err(ExperienceError::InvalidSnapshot);
        }
        // Validate every item and identity before mutating memory. Import is
        // therefore an atomic contract at the API boundary.
        let mut identities = BTreeSet::new();
        for experience in &snapshot.experiences {
            validate_experience(experience)?;
            if !identities.insert(&experience.experience_id) {
                return Err(ExperienceError::InvalidSnapshot);
            }
            if self
                .records
                .get(&experience.experience_id)
                .is_some_and(|(sha256, _)| sha256 != &experience_sha256(experience))
            {
                return Err(ExperienceError::IdentityConflict);
            }
        }
        snapshot
            .experiences
            .iter()
            .cloned()
            .map(|experience| self.inject(experience))
            .collect()
    }
}

fn validate_experience(experience: &ExperienceIR) -> Result<(), ExperienceError> {
    if experience.schema != EXPERIENCE_SCHEMA {
        return Err(ExperienceError::InvalidSchema);
    }
    if experience.experience_id.is_empty()
        || experience.experience_id.len() > 128
        || !experience
            .experience_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(ExperienceError::InvalidIdentity);
    }
    if [
        &experience.situation,
        &experience.action,
        &experience.outcome_description,
    ]
    .iter()
    .any(|value| value.trim().is_empty() || value.len() > MAX_EXPERIENCE_TEXT_BYTES)
    {
        return Err(ExperienceError::InvalidText);
    }
    if experience.confidence_millis > 1_000 {
        return Err(ExperienceError::InvalidConfidence);
    }
    if experience.semantic_tags.len() > MAX_TAGS {
        return Err(ExperienceError::TooManyTags);
    }
    if experience
        .semantic_tags
        .iter()
        .any(|tag| tag.trim().is_empty() || tag.len() > 128)
    {
        return Err(ExperienceError::InvalidText);
    }
    if experience.evidence.len() > MAX_EVIDENCE_ITEMS
        || experience
            .evidence
            .iter()
            .any(|evidence| evidence.trim().is_empty() || evidence.len() > 4_096)
    {
        return Err(ExperienceError::TooManyEvidenceItems);
    }
    if experience
        .source_language
        .as_ref()
        .is_some_and(|language| language.trim().is_empty() || language.len() > 32)
    {
        return Err(ExperienceError::InvalidText);
    }
    Ok(())
}

fn experience_sha256(experience: &ExperienceIR) -> String {
    format!(
        "{:X}",
        Sha256::digest(serde_json::to_vec(experience).unwrap_or_default())
    )
}

fn experience_sequence_sha256(experiences: &[ExperienceIR]) -> String {
    format!(
        "{:X}",
        Sha256::digest(serde_json::to_vec(experiences).unwrap_or_default())
    )
}

fn normalized_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn experience(id: &str, action: &str) -> ExperienceIR {
        ExperienceIR {
            schema: EXPERIENCE_SCHEMA.to_string(),
            experience_id: id.to_string(),
            situation: "PowerShell UNC path command failed".to_string(),
            action: action.to_string(),
            outcome: ExperienceOutcomeIR::Successful,
            outcome_description: "PathBuf and literal path preserved the target".to_string(),
            semantic_tags: vec!["powershell".to_string(), "path".to_string()],
            evidence: vec!["exit_code=0".to_string()],
            confidence_millis: 950,
            source_language: Some("en".to_string()),
        }
    }

    #[test]
    fn injection_is_content_addressed_and_identity_safe() {
        let mut memory = ExperienceMemory::new(2);
        let first = memory
            .inject(experience("EXP-1", "use literal path"))
            .unwrap();
        assert!(first.inserted);
        assert!(
            !memory
                .inject(experience("EXP-1", "use literal path"))
                .unwrap()
                .inserted
        );
        assert_eq!(
            memory.inject(experience("EXP-1", "change directory first")),
            Err(ExperienceError::IdentityConflict)
        );
    }

    #[test]
    fn recall_uses_typed_tags_and_text_terms() {
        let mut memory = ExperienceMemory::new(2);
        memory
            .inject(experience("EXP-1", "use literal path"))
            .unwrap();
        let recalled = memory
            .recall(&ExperienceQueryIR {
                semantic_tags: vec!["path".to_string()],
                text_terms: vec!["powershell".to_string()],
                preferred_outcome: Some(ExperienceOutcomeIR::Successful),
                max_results: 1,
            })
            .unwrap();
        assert_eq!(recalled[0].experience.experience_id, "EXP-1");
        assert!(recalled[0].relevance_score > 100);
    }

    #[test]
    fn snapshot_round_trip_is_content_addressed_and_tamper_evident() {
        let mut source = ExperienceMemory::new(2);
        source
            .inject(experience("EXP-1", "use literal path"))
            .unwrap();
        let snapshot = source.export_snapshot();
        let mut destination = ExperienceMemory::new(2);
        destination.import_snapshot(&snapshot).unwrap();
        assert_eq!(destination.export_snapshot(), snapshot);
        let mut tampered = snapshot;
        tampered.experiences[0].action = "unverified alternative".to_string();
        assert_eq!(
            ExperienceMemory::new(2).import_snapshot(&tampered),
            Err(ExperienceError::InvalidSnapshot)
        );
    }
}
