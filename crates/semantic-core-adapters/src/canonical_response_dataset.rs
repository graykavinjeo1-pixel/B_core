//! Training-record boundary for the language codec.
//!
//! A record contains only canonical approved meaning and a surface realized
//! from that same meaning. Request text, parser state, hidden state, and prior
//! generated responses never cross this boundary.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::approved_response::ApprovedCompositionalResponseIR;
use crate::document_response::{realize_document_response, DocumentResponseOutputIR};
use crate::language_knowledge::LanguageCodeIR;

pub const CANONICAL_RESPONSE_TRAINING_RECORD_SCHEMA: &str =
    "BCORE.CANONICAL_RESPONSE_TRAINING_RECORD.V1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CanonicalResponseDatasetSplitIR {
    Train,
    Validation,
    Blind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalResponseTrainingRecordIR {
    pub schema: String,
    pub record_id: String,
    pub split: CanonicalResponseDatasetSplitIR,
    pub language: LanguageCodeIR,
    pub approved_response: ApprovedCompositionalResponseIR,
    /// Surface generated only from the approved response; it is a reference
    /// for codec fitting, never an input to the student.
    pub surface_reference: String,
    pub document_output_sha256: String,
    pub record_sha256: String,
}

impl CanonicalResponseTrainingRecordIR {
    pub fn from_approved(
        approved_response: ApprovedCompositionalResponseIR,
        language: LanguageCodeIR,
    ) -> Result<Self, String> {
        let split = split_for(&approved_response.semantic_sha256);
        Self::from_approved_with_split(approved_response, language, split)
    }

    /// The campaign owns its semantic-disjoint split before realization.
    /// This avoids assigning siblings from one synthetic world family to both
    /// training and blind evaluation through an output hash accident.
    pub fn from_approved_with_split(
        approved_response: ApprovedCompositionalResponseIR,
        language: LanguageCodeIR,
        split: CanonicalResponseDatasetSplitIR,
    ) -> Result<Self, String> {
        if !approved_response.validate()
            || !matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::English)
        {
            return Err("INVALID_CANONICAL_RESPONSE_TRAINING_SOURCE".into());
        }
        let document = realize_document_response(&approved_response, language)
            .map_err(|error| format!("CANONICAL_RESPONSE_SURFACE_FAILED:{error}"))?;
        Self::from_document(approved_response, language, split, document)
    }

    fn from_document(
        approved_response: ApprovedCompositionalResponseIR,
        language: LanguageCodeIR,
        split: CanonicalResponseDatasetSplitIR,
        document: DocumentResponseOutputIR,
    ) -> Result<Self, String> {
        if !document.validate(&approved_response)
            || document.unsupported_claims != 0
            || document.markdown.trim().is_empty()
            || document.markdown.chars().count() > 16_384
            || contains_internal_symbol(&document.markdown)
        {
            return Err("INVALID_CANONICAL_RESPONSE_SURFACE".into());
        }
        let semantic_prefix = &approved_response.semantic_sha256[..24];
        let mut record = Self {
            schema: CANONICAL_RESPONSE_TRAINING_RECORD_SCHEMA.into(),
            record_id: format!("CANONICAL-RESPONSE:{semantic_prefix}:{language:?}"),
            split,
            language,
            approved_response,
            surface_reference: document.markdown,
            document_output_sha256: document.output_sha256,
            record_sha256: String::new(),
        };
        record.record_sha256 = canonical_response_training_record_sha256(&record);
        record
            .validate()
            .then_some(record)
            .ok_or_else(|| "CANONICAL_RESPONSE_TRAINING_RECORD_INVALID".into())
    }

    pub fn validate(&self) -> bool {
        self.schema == CANONICAL_RESPONSE_TRAINING_RECORD_SCHEMA
            && !self.record_id.trim().is_empty()
            && matches!(
                self.language,
                LanguageCodeIR::Korean | LanguageCodeIR::English
            )
            && self.approved_response.validate()
            && !self.surface_reference.trim().is_empty()
            && !contains_internal_symbol(&self.surface_reference)
            && sha256_shape(&self.document_output_sha256)
            && sha256_shape(&self.record_sha256)
            && self.record_sha256 == canonical_response_training_record_sha256(self)
    }
}

pub fn canonical_response_training_record_sha256(
    record: &CanonicalResponseTrainingRecordIR,
) -> String {
    let mut canonical = record.clone();
    canonical.record_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&canonical).expect("canonical training record serializes"),
        )
    )
}

fn split_for(semantic_sha256: &str) -> CanonicalResponseDatasetSplitIR {
    match u8::from_str_radix(&semantic_sha256[..2], 16).unwrap_or_default() % 20 {
        0 => CanonicalResponseDatasetSplitIR::Blind,
        1 => CanonicalResponseDatasetSplitIR::Validation,
        _ => CanonicalResponseDatasetSplitIR::Train,
    }
}

fn contains_internal_symbol(surface: &str) -> bool {
    surface
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .any(|token| {
            token.starts_with("C_")
                || token.starts_with("NATIVE_")
                || token.starts_with("CANONICAL_")
                || token.starts_with("SEMANTIC_")
        })
}

fn sha256_shape(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
