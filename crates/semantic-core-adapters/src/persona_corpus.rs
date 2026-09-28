//! Provenance-bound ingress for persona language observations.
//!
//! A corpus sentence is never a semantic authority.  It can become a lexical
//! phenotype only after a reviewer binds it to an already approved semantic
//! concept and supplies morphology, register, and persona tags.

use crate::{
    affective_field::{RoleplayPersonaIR, RoleplayRelationshipIR, RoleplayVoiceIR},
    generative_language::{
        ExpressionMorphologyClassIR, ExpressionNodeIR, ExpressionPartOfSpeechIR,
        GenerationEmotionIR,
    },
    language_knowledge::LanguageCodeIR,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PERSONA_CORPUS_RECORD_SCHEMA: &str = "BCORE.PERSONA_CORPUS_RECORD.V1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PersonaCorpusRightsIR {
    /// The source may be examined and used for internal research only.
    ResearchOnly,
    /// The source is available only under a deployment-specific agreement.
    /// The agreement reference belongs in the import manifest and provenance.
    ContractBound,
    /// The reviewed source is permitted for the configured production use.
    ProductionApproved,
    /// The data must not enter either corpus compilation or production.
    #[default]
    PendingReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaCorpusProvenanceIR {
    pub source_id: String,
    pub source_revision: String,
    pub rights: PersonaCorpusRightsIR,
    pub review_reference: String,
}

impl PersonaCorpusProvenanceIR {
    pub fn validate(&self) -> bool {
        !self.source_id.trim().is_empty()
            && !self.source_revision.trim().is_empty()
            && !self.review_reference.trim().is_empty()
            && self.source_id.len() <= 160
            && self.source_revision.len() <= 160
            && self.review_reference.len() <= 512
    }
}

/// A reviewed corpus observation, normalized into a word or phrase candidate.
/// `semantic_concept_id` identifies an existing B_Core meaning; it is not
/// derived from the corpus surface automatically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaCorpusRecordIR {
    pub schema: String,
    pub record_id: String,
    pub provenance: PersonaCorpusProvenanceIR,
    pub language: LanguageCodeIR,
    pub semantic_concept_id: String,
    pub lexical_root: String,
    pub part_of_speech: ExpressionPartOfSpeechIR,
    pub morphology: ExpressionMorphologyClassIR,
    pub register: crate::language_knowledge::LanguageRegisterIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_emotion: Option<GenerationEmotionIR>,
    pub persona: RoleplayPersonaIR,
    pub confidence_millis: u16,
    /// A B_Core review/audit reference which binds this lexical candidate to
    /// the selected pre-existing semantic concept.
    pub semantic_review_reference: String,
    pub record_sha256: String,
}

/// Unsealed input for one reviewed corpus observation. `from_draft` seals it
/// with the schema and content hash before it can enter an expression store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaCorpusRecordDraftIR {
    pub record_id: String,
    pub provenance: PersonaCorpusProvenanceIR,
    pub language: LanguageCodeIR,
    pub semantic_concept_id: String,
    pub lexical_root: String,
    pub part_of_speech: ExpressionPartOfSpeechIR,
    pub morphology: ExpressionMorphologyClassIR,
    pub register: crate::language_knowledge::LanguageRegisterIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_emotion: Option<GenerationEmotionIR>,
    pub persona: RoleplayPersonaIR,
    pub confidence_millis: u16,
    pub semantic_review_reference: String,
}

impl PersonaCorpusRecordIR {
    pub fn from_draft(draft: PersonaCorpusRecordDraftIR) -> Self {
        let mut record = Self {
            schema: PERSONA_CORPUS_RECORD_SCHEMA.to_string(),
            record_id: draft.record_id,
            provenance: draft.provenance,
            language: draft.language,
            semantic_concept_id: draft.semantic_concept_id,
            lexical_root: draft.lexical_root,
            part_of_speech: draft.part_of_speech,
            morphology: draft.morphology,
            register: draft.register,
            preferred_emotion: draft.preferred_emotion,
            persona: draft.persona,
            confidence_millis: draft.confidence_millis,
            semantic_review_reference: draft.semantic_review_reference,
            record_sha256: String::new(),
        };
        record.record_sha256 = persona_corpus_record_sha256(&record);
        record
    }

    pub fn validate(&self) -> bool {
        self.schema == PERSONA_CORPUS_RECORD_SCHEMA
            && self.provenance.validate()
            && !matches!(
                self.language,
                LanguageCodeIR::Mixed | LanguageCodeIR::Unknown
            )
            && !self.record_id.trim().is_empty()
            && !self.semantic_concept_id.trim().is_empty()
            && !self.lexical_root.trim().is_empty()
            && !self.semantic_review_reference.trim().is_empty()
            && self.confidence_millis <= 1_000
            && self.lexical_root.len() <= 240
            && !self
                .lexical_root
                .contains(['.', '!', '?', '。', '！', '？'])
            && valid_sha256(&self.record_sha256)
            && self.record_sha256 == persona_corpus_record_sha256(self)
    }

    /// Compile an approved lexical phenotype. Production compilation rejects
    /// observations whose provenance has not been explicitly cleared.
    pub fn compile_expression(&self, production: bool) -> Result<ExpressionNodeIR, String> {
        if !self.validate() {
            return Err("INVALID_PERSONA_CORPUS_RECORD".to_string());
        }
        if production && self.provenance.rights != PersonaCorpusRightsIR::ProductionApproved {
            return Err("PERSONA_CORPUS_RIGHTS_NOT_APPROVED".to_string());
        }
        Ok(ExpressionNodeIR {
            preferred_emotion: self.preferred_emotion,
            preferred_korean_dialect: None,
            preferred_roleplay_relationship: match self.persona.relationship {
                RoleplayRelationshipIR::Unspecified => None,
                relationship => Some(relationship),
            },
            preferred_roleplay_voice: match self.persona.voice {
                RoleplayVoiceIR::Balanced => None,
                voice => Some(voice),
            },
            korean_nominal_forms: Vec::new(),
            expression_id: format!("EXPR.PERSONA.CORPUS.{}", self.record_sha256),
            language: self.language,
            concept_id: self.semantic_concept_id.clone(),
            lexical_root: self.lexical_root.clone(),
            part_of_speech: self.part_of_speech,
            morphology: self.morphology,
            register: self.register,
            confidence_millis: self.confidence_millis,
            provenance: format!(
                "PERSONA_CORPUS:{}:{}:{}",
                self.provenance.source_id,
                self.provenance.source_revision,
                self.semantic_review_reference
            ),
        })
    }
}

pub fn persona_corpus_record_sha256(record: &PersonaCorpusRecordIR) -> String {
    let mut canonical = record.clone();
    canonical.record_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).expect("persona corpus record serializes"))
    )
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        affective_field::{RoleplayRelationshipIR as Relationship, RoleplayVoiceIR as Voice},
        language_knowledge::LanguageRegisterIR,
    };

    fn record(rights: PersonaCorpusRightsIR) -> PersonaCorpusRecordIR {
        PersonaCorpusRecordIR::from_draft(PersonaCorpusRecordDraftIR {
            record_id: "SMILESTYLE-GREETING-001".into(),
            provenance: PersonaCorpusProvenanceIR {
                source_id: "SMILESTYLE".into(),
                source_revision: "2022-REVIEWED-SNAPSHOT".into(),
                rights,
                review_reference: "LICENSE-REVIEW:SMILESTYLE-001".into(),
            },
            language: LanguageCodeIR::Korean,
            semantic_concept_id: "C_DIALOGUE_GREETING_REPLY".into(),
            lexical_root: "반가워".into(),
            part_of_speech: ExpressionPartOfSpeechIR::Interjection,
            morphology: ExpressionMorphologyClassIR::KoreanInvariable,
            register: LanguageRegisterIR::Informal,
            preferred_emotion: None,
            persona: RoleplayPersonaIR {
                relationship: Relationship::Close,
                voice: Voice::Gentle,
                ..Default::default()
            },
            confidence_millis: 1_000,
            semantic_review_reference: "SEMANTIC-REVIEW:C_DIALOGUE_GREETING_REPLY:V1".into(),
        })
    }

    #[test]
    fn compiler_preserves_persona_tags_but_blocks_unapproved_production_data() {
        let pending = record(PersonaCorpusRightsIR::PendingReview);
        assert!(pending.validate());
        assert_eq!(
            pending.compile_expression(true),
            Err("PERSONA_CORPUS_RIGHTS_NOT_APPROVED".into())
        );
        let expression = pending.compile_expression(false).unwrap();
        assert_eq!(expression.concept_id, "C_DIALOGUE_GREETING_REPLY");
        assert_eq!(expression.lexical_root, "반가워");
        assert_eq!(
            expression.preferred_roleplay_relationship,
            Some(Relationship::Close)
        );
        assert_eq!(expression.preferred_roleplay_voice, Some(Voice::Gentle));
        assert!(expression
            .provenance
            .starts_with("PERSONA_CORPUS:SMILESTYLE:"));

        let mut tampered = record(PersonaCorpusRightsIR::ProductionApproved);
        tampered.semantic_concept_id = "C_UNRELATED_ACTION".into();
        assert!(!tampered.validate());
        assert_eq!(
            tampered.compile_expression(false),
            Err("INVALID_PERSONA_CORPUS_RECORD".into())
        );
    }
}
