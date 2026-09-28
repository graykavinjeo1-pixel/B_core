//! Streaming, provenance-preserving ingestion for external persona corpora.
//!
//! These adapters deliberately stop before semantic compilation.  A raw
//! conversation demonstrates a possible surface style; it never assigns a
//! B_Core concept, truth value, or response plan on its own.  Only a reviewed
//! `PersonaCorpusRecordIR` may cross into the language codec.

use crate::{
    affective_field::RoleplayPersonaIR,
    language_knowledge::LanguageCodeIR,
    persona_corpus::{PersonaCorpusProvenanceIR, PersonaCorpusRightsIR},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::{self, BufRead},
};

pub const PERSONA_CORPUS_IMPORT_MANIFEST_SCHEMA: &str = "BCORE.PERSONA_CORPUS_IMPORT_MANIFEST.V1";
pub const PIPPA_DEDUPED_SOURCE_ID: &str = "PYGMALION_PIPPA_DEDUPED";
pub const NIKL_EVERYONES_CORPUS_SOURCE_ID: &str = "NIKL_EVERYONES_CORPUS";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PersonaCorpusSourceIR {
    PippaDeduped,
    NiklEveryonesCorpus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PersonaCorpusAudienceIR {
    General,
    AdultOnly,
}

/// A source-use receipt. It proves the local bytes and the intended service
/// boundary, but does not claim that an unreviewed utterance is safe or true.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaCorpusImportManifestIR {
    pub schema: String,
    pub source: PersonaCorpusSourceIR,
    pub source_revision: String,
    pub raw_sha256: String,
    pub rights: PersonaCorpusRightsIR,
    pub audience: PersonaCorpusAudienceIR,
    pub domestic_service_only: bool,
    /// PIPPA: published licence review. NIKL: the approved local agreement.
    pub authorization_reference: String,
    pub manifest_sha256: String,
}

impl PersonaCorpusImportManifestIR {
    pub fn new(
        source: PersonaCorpusSourceIR,
        source_revision: String,
        raw_sha256: String,
        rights: PersonaCorpusRightsIR,
        audience: PersonaCorpusAudienceIR,
        domestic_service_only: bool,
        authorization_reference: String,
    ) -> Self {
        let mut manifest = Self {
            schema: PERSONA_CORPUS_IMPORT_MANIFEST_SCHEMA.to_string(),
            source,
            source_revision,
            raw_sha256,
            rights,
            audience,
            domestic_service_only,
            authorization_reference,
            manifest_sha256: String::new(),
        };
        manifest.manifest_sha256 = persona_corpus_import_manifest_sha256(&manifest);
        manifest
    }

    pub fn validate(&self) -> bool {
        let source_constraints_hold = match self.source {
            // PIPPA is explicitly an adult-only source in this product. Its
            // license is public, but every extracted surface still enters the
            // semantic/safety review queue before codec use.
            PersonaCorpusSourceIR::PippaDeduped => {
                self.rights == PersonaCorpusRightsIR::ProductionApproved
                    && self.audience == PersonaCorpusAudienceIR::AdultOnly
            }
            // The NIKL portal grants access under a user-specific agreement;
            // source bytes cannot be treated as generally redistributable.
            PersonaCorpusSourceIR::NiklEveryonesCorpus => {
                self.rights == PersonaCorpusRightsIR::ContractBound && self.domestic_service_only
            }
        };
        self.schema == PERSONA_CORPUS_IMPORT_MANIFEST_SCHEMA
            && source_constraints_hold
            && !self.source_revision.trim().is_empty()
            && valid_sha256(&self.raw_sha256)
            && !self.authorization_reference.trim().is_empty()
            && self.source_id() != "UNKNOWN"
            && self.manifest_sha256 == persona_corpus_import_manifest_sha256(self)
    }

    pub fn source_id(&self) -> &'static str {
        match self.source {
            PersonaCorpusSourceIR::PippaDeduped => PIPPA_DEDUPED_SOURCE_ID,
            PersonaCorpusSourceIR::NiklEveryonesCorpus => NIKL_EVERYONES_CORPUS_SOURCE_ID,
        }
    }

    pub fn provenance(&self, review_reference: String) -> PersonaCorpusProvenanceIR {
        PersonaCorpusProvenanceIR {
            source_id: self.source_id().to_string(),
            source_revision: self.source_revision.clone(),
            rights: self.rights,
            review_reference,
        }
    }
}

pub fn persona_corpus_import_manifest_sha256(manifest: &PersonaCorpusImportManifestIR) -> String {
    let mut canonical = manifest.clone();
    canonical.manifest_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).expect("persona corpus manifest serializes"))
    )
}

/// One bot turn awaiting semantic and safety review. The raw surface is only
/// exposed to the caller's local review sink; this module never treats it as
/// an expression node or writes it to the executable package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonaCorpusReviewObservationIR {
    pub source_record_id: String,
    pub persona_id: String,
    pub persona_label: String,
    pub language: LanguageCodeIR,
    pub turn_index: u32,
    pub surface: String,
    pub surface_sha256: String,
    pub suggested_persona: RoleplayPersonaIR,
}

/// Receipt produced while streaming PIPPA. No dialogue text is retained in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PippaIngestReportIR {
    pub source_sha256: String,
    pub conversation_count: u32,
    pub ignored_short_conversation_count: u32,
    pub bot_turn_count: u32,
    pub human_turn_count: u32,
    pub ignored_empty_turn_count: u32,
    pub distinct_persona_count: u32,
    pub report_sha256: String,
}

impl PippaIngestReportIR {
    fn seal(
        source_sha256: String,
        conversation_count: u32,
        ignored_short_conversation_count: u32,
        bot_turn_count: u32,
        human_turn_count: u32,
        ignored_empty_turn_count: u32,
        distinct_persona_count: u32,
    ) -> Self {
        let mut report = Self {
            source_sha256,
            conversation_count,
            ignored_short_conversation_count,
            bot_turn_count,
            human_turn_count,
            ignored_empty_turn_count,
            distinct_persona_count,
            report_sha256: String::new(),
        };
        let mut canonical = report.clone();
        canonical.report_sha256.clear();
        report.report_sha256 = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&canonical).expect("PIPPA report serializes"))
        );
        report
    }

    pub fn validate(&self) -> bool {
        let resealed = Self::seal(
            self.source_sha256.clone(),
            self.conversation_count,
            self.ignored_short_conversation_count,
            self.bot_turn_count,
            self.human_turn_count,
            self.ignored_empty_turn_count,
            self.distinct_persona_count,
        );
        valid_sha256(&self.source_sha256)
            && self.conversation_count > 0
            && self.bot_turn_count > 0
            && self.report_sha256 == resealed.report_sha256
    }
}

/// Stream the official `pippa_deduped.jsonl` form without loading it into
/// memory. `on_bot_turn` owns review-queue storage and can reject a candidate;
/// no raw corpus string is silently promoted into the codec.
pub fn stream_pippa_deduped<R, F>(
    reader: R,
    mut on_bot_turn: F,
) -> Result<PippaIngestReportIR, String>
where
    R: BufRead,
    F: FnMut(PersonaCorpusReviewObservationIR) -> Result<(), String>,
{
    let mut digest = Sha256::new();
    let mut line = Vec::new();
    let mut conversation_count = 0_u32;
    let mut ignored_short_conversation_count = 0_u32;
    let mut bot_turn_count = 0_u32;
    let mut human_turn_count = 0_u32;
    let mut ignored_empty_turn_count = 0_u32;
    let mut personas = BTreeSet::new();
    let mut reader = reader;

    loop {
        line.clear();
        let bytes = reader.read_until(b'\n', &mut line).map_err(io_error)?;
        if bytes == 0 {
            break;
        }
        digest.update(&line);
        let trimmed = trim_line_end(&line);
        if trimmed.is_empty() {
            return Err("PIPPA_EMPTY_LINE".to_string());
        }
        let value: serde_json::Value =
            serde_json::from_slice(trimmed).map_err(|_| "PIPPA_INVALID_JSONL".to_string())?;
        let bot_id = required_string(&value, "bot_id")?;
        let bot_name = required_string(&value, "bot_name")?;
        let conversation = value
            .get("conversation")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "PIPPA_CONVERSATION_MISSING".to_string())?;
        let record_index = conversation_count;
        conversation_count = conversation_count
            .checked_add(1)
            .ok_or_else(|| "PIPPA_CONVERSATION_COUNT_OVERFLOW".to_string())?;
        personas.insert(bot_id.to_string());
        // The public deduped file can still contain a small number of short
        // conversations. Account for them deterministically, but do not send
        // them to persona review because they lack interaction context.
        if conversation.len() < 3 {
            ignored_short_conversation_count = ignored_short_conversation_count
                .checked_add(1)
                .ok_or_else(|| "PIPPA_CONVERSATION_COUNT_OVERFLOW".to_string())?;
            continue;
        }

        for (turn_index, turn) in conversation.iter().enumerate() {
            let message = string_field(turn, "message")?;
            if message.len() > 32_768 {
                return Err("PIPPA_TURN_TOO_LONG".to_string());
            }
            let is_human = turn
                .get("is_human")
                .and_then(serde_json::Value::as_bool)
                .ok_or_else(|| "PIPPA_TURN_ROLE_MISSING".to_string())?;
            if is_human {
                human_turn_count = human_turn_count
                    .checked_add(1)
                    .ok_or_else(|| "PIPPA_TURN_COUNT_OVERFLOW".to_string())?;
                continue;
            }
            bot_turn_count = bot_turn_count
                .checked_add(1)
                .ok_or_else(|| "PIPPA_TURN_COUNT_OVERFLOW".to_string())?;
            // A valid JSON row may carry a deliberately empty assistant
            // message. It contributes to source accounting but cannot become
            // a lexical phenotype candidate.
            if message.trim().is_empty() {
                ignored_empty_turn_count = ignored_empty_turn_count
                    .checked_add(1)
                    .ok_or_else(|| "PIPPA_TURN_COUNT_OVERFLOW".to_string())?;
                continue;
            }
            on_bot_turn(PersonaCorpusReviewObservationIR {
                source_record_id: format!("PIPPA:{record_index}:{turn_index}"),
                persona_id: bot_id.to_string(),
                persona_label: bot_name.to_string(),
                language: LanguageCodeIR::English,
                turn_index: turn_index as u32,
                surface: message.to_string(),
                surface_sha256: format!("{:x}", Sha256::digest(message.as_bytes())),
                suggested_persona: RoleplayPersonaIR::default(),
            })?;
        }
    }
    Ok(PippaIngestReportIR::seal(
        format!("{:x}", digest.finalize()),
        conversation_count,
        ignored_short_conversation_count,
        bot_turn_count,
        human_turn_count,
        ignored_empty_turn_count,
        personas.len() as u32,
    ))
}

fn required_string<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    let field = string_field(value, key)?;
    if field.trim().is_empty() {
        Err(format!("PIPPA_{key}_MISSING"))
    } else {
        Ok(field)
    }
}

fn string_field<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("PIPPA_{key}_MISSING"))
}

fn trim_line_end(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r\n")
        .or_else(|| line.strip_suffix(b"\n"))
        .unwrap_or(line)
}

fn io_error(error: io::Error) -> String {
    format!("PIPPA_IO:{error}")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const FIXTURE: &str = concat!(
        r#"{"bot_id":"unit-persona","bot_name":"Unit Persona","conversation":[{"message":"Hello","is_human":false},{"message":"Hi","is_human":true},{"message":"How can I help?","is_human":false}]}"#,
        "\n"
    );

    #[test]
    fn pippa_stream_is_quarantined_until_a_reviewer_binds_meaning() {
        let mut seen = Vec::new();
        let report = stream_pippa_deduped(Cursor::new(FIXTURE), |observation| {
            seen.push(observation);
            Ok(())
        })
        .unwrap();
        assert!(report.validate());
        assert_eq!(report.conversation_count, 1);
        assert_eq!(report.bot_turn_count, 2);
        assert_eq!(report.human_turn_count, 1);
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].language, LanguageCodeIR::English);
        assert!(seen[0].surface_sha256.len() == 64);
    }

    #[test]
    fn source_manifests_encode_the_actual_service_boundaries() {
        let hash = "a".repeat(64);
        let pippa = PersonaCorpusImportManifestIR::new(
            PersonaCorpusSourceIR::PippaDeduped,
            "pippa_deduped.jsonl@main".into(),
            hash.clone(),
            PersonaCorpusRightsIR::ProductionApproved,
            PersonaCorpusAudienceIR::AdultOnly,
            false,
            "LICENSE-REVIEW:APACHE-2.0".into(),
        );
        assert!(pippa.validate());
        assert_eq!(
            pippa.provenance("REVIEW:PIPPA:1".into()).source_id,
            PIPPA_DEDUPED_SOURCE_ID
        );

        let nikl = PersonaCorpusImportManifestIR::new(
            PersonaCorpusSourceIR::NiklEveryonesCorpus,
            "APPROVED-SNAPSHOT-2026-09".into(),
            hash,
            PersonaCorpusRightsIR::ContractBound,
            PersonaCorpusAudienceIR::General,
            true,
            "NIKL-AGREEMENT:LOCAL-APPROVAL".into(),
        );
        assert!(nikl.validate());
        assert!(!PersonaCorpusImportManifestIR::new(
            PersonaCorpusSourceIR::NiklEveryonesCorpus,
            "snapshot".into(),
            "b".repeat(64),
            PersonaCorpusRightsIR::ProductionApproved,
            PersonaCorpusAudienceIR::General,
            true,
            "not-an-agreement".into(),
        )
        .validate());
    }
}
