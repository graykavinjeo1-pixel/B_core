//! Durable, shadow-safe runtime for learned language construction profiles.
//!
//! A loaded artifact licenses only a style profile already observed in the
//! training split.  It cannot add, remove, negate, or reinterpret approved
//! meaning.  The document codec realizes the projected style and its inverse
//! is checked before the response is returned.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::affective_field::{
    KoreanDialectIR, RoleplayAgeBandIR, RoleplayPersonaIR, RoleplayRelationshipIR, RoleplaySceneIR,
    RoleplayVoiceIR,
};
use crate::approved_response::{
    compositional_response_sha256, ApprovedCompositionalResponseIR, ApprovedOpenValueIR,
    ApprovedRelationTypeIR, ApprovedResponseStyleIR, ApprovedVerbosityIR,
};
use crate::canonical_response_training::CanonicalResponseCodecModelIR;
use crate::document_response::{
    interpret_document_semantics, realize_document_response, DocumentSemanticInterpretationIR,
};
use crate::language_knowledge::{LanguageCodeIR, LanguageRegisterIR};
use crate::persona_contrastive_training::{
    construction_key_sha256_for_response, ExpressionConstructionInventoryArtifactIR,
    LanguageStateControllerArtifactIR, LanguageStateProjectionIR,
    PersonaContrastiveConstructionModelIR, PersonaLatentControlsIR,
    PersonaLatentOperatorArtifactIR,
};

pub const PERSONA_CONSTRUCTION_OUTPUT_SCHEMA: &str = "BCORE.PERSONA_CONSTRUCTION_OUTPUT.V1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PersonaConstructionModeIR {
    DefaultPolite,
    FriendlyPolite,
    Concise,
    Explanatory,
    CalmSupportive,
}

/// Explicit B_Core state routed into language realization.  These values are
/// state observations or caller-supplied conditions, never semantic authority.
/// The current construction codec preserves them at the boundary but does not
/// add state-specific surface rules yet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageStateConditionIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_emotion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inferred_user_emotion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship: Option<RoleplayRelationshipIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialogue_context: Option<RoleplaySceneIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_band: Option<RoleplayAgeBandIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub register: Option<LanguageRegisterIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub korean_dialect: Option<KoreanDialectIR>,
}

impl LanguageStateConditionIR {
    pub fn validate(&self) -> bool {
        self.personality
            .as_deref()
            .map_or(true, |value| !value.is_empty() && value.len() <= 64)
            && self
                .speaker_emotion
                .as_deref()
                .map_or(true, |value| !value.is_empty() && value.len() <= 64)
            && self
                .inferred_user_emotion
                .as_deref()
                .map_or(true, |value| !value.is_empty() && value.len() <= 64)
    }
}

/// Explicit runtime social/pragmatic state. It is supplied by B_Core's
/// dialogue state or the caller; no demographic attribute is inferred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaConstructionContextIR {
    pub mode: PersonaConstructionModeIR,
    #[serde(default)]
    pub persona: RoleplayPersonaIR,
    /// Explicit language-profile selector. It is not semantic authority and
    /// is never inferred from the approved response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona_label: Option<String>,
    /// Continuous candidate used only for language realization. The approved
    /// meaning remains unchanged; range is -1000..=1000 (-1.0..=+1.0).
    #[serde(default)]
    pub verbosity_delta_millis: i16,
    /// Opaque research intervention coordinates. They are interpreted only by
    /// the loaded latent artifact and never become semantic authority.
    #[serde(default)]
    pub latent_operator_deltas_millis: Vec<i16>,
    /// Explicit multi-factor state routed to the codec boundary.  It is kept
    /// separate from persona labels so no axis becomes a hidden template key.
    #[serde(default)]
    pub language_state: LanguageStateConditionIR,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PersonaDiscourseMarkerIR {
    None,
    FriendlyOrientation,
    SupportiveOrientation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaConstructionOutputIR {
    pub schema: String,
    pub source_response_sha256: String,
    pub projected_response_sha256: String,
    pub model_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contrastive_model_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contrastive_profile: Option<String>,
    pub context: PersonaConstructionContextIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_state_projection: Option<LanguageStateProjectionIR>,
    pub marker: PersonaDiscourseMarkerIR,
    pub markdown: String,
    /// The inverse is computed from the semantic body after stripping the
    /// registered non-factual marker, never from a guessed surface parse.
    pub semantic_inverse: DocumentSemanticInterpretationIR,
    pub output_sha256: String,
}

#[derive(Debug, Clone)]
pub struct ConstructionRuntime {
    model: CanonicalResponseCodecModelIR,
    contrastive_model: Option<PersonaContrastiveConstructionModelIR>,
    latent_model: Option<PersonaLatentOperatorArtifactIR>,
    language_state_controller: Option<LanguageStateControllerArtifactIR>,
    expression_inventory: Option<ExpressionConstructionInventoryArtifactIR>,
}

impl ConstructionRuntime {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|_| "CONSTRUCTION_ARTIFACT_READ_FAILED")?;
        let model = serde_json::from_slice::<CanonicalResponseCodecModelIR>(&bytes)
            .map_err(|_| "CONSTRUCTION_ARTIFACT_JSON_INVALID")?;
        model
            .validate()
            .then_some(Self {
                model,
                contrastive_model: None,
                latent_model: None,
                language_state_controller: None,
                expression_inventory: None,
            })
            .ok_or_else(|| "CONSTRUCTION_ARTIFACT_INVALID".into())
    }

    pub fn from_model(model: CanonicalResponseCodecModelIR) -> Result<Self, String> {
        model
            .validate()
            .then_some(Self {
                model,
                contrastive_model: None,
                latent_model: None,
                language_state_controller: None,
                expression_inventory: None,
            })
            .ok_or_else(|| "CONSTRUCTION_MODEL_INVALID".into())
    }

    pub fn load_from_paths(
        construction_path: impl AsRef<Path>,
        contrastive_path: impl AsRef<Path>,
    ) -> Result<Self, String> {
        let mut runtime = Self::load_from_path(construction_path)?;
        runtime.contrastive_model = Some(PersonaContrastiveConstructionModelIR::load_from_path(
            contrastive_path,
        )?);
        Ok(runtime)
    }

    pub fn load_from_paths_with_latent(
        construction_path: impl AsRef<Path>,
        contrastive_path: impl AsRef<Path>,
        latent_path: impl AsRef<Path>,
    ) -> Result<Self, String> {
        let mut runtime = Self::load_from_paths(construction_path, contrastive_path)?;
        runtime.latent_model = Some(PersonaLatentOperatorArtifactIR::load_from_path(
            latent_path,
        )?);
        Ok(runtime)
    }

    pub fn load_from_paths_with_latent_and_controller(
        construction_path: impl AsRef<Path>,
        contrastive_path: impl AsRef<Path>,
        latent_path: impl AsRef<Path>,
        controller_path: impl AsRef<Path>,
    ) -> Result<Self, String> {
        let mut runtime =
            Self::load_from_paths_with_latent(construction_path, contrastive_path, latent_path)?;
        runtime.language_state_controller = Some(
            LanguageStateControllerArtifactIR::load_from_path(controller_path)?,
        );
        Ok(runtime)
    }

    pub fn load_from_paths_with_latent_controller_and_inventory(
        construction_path: impl AsRef<Path>,
        contrastive_path: impl AsRef<Path>,
        latent_path: impl AsRef<Path>,
        controller_path: impl AsRef<Path>,
        inventory_path: impl AsRef<Path>,
    ) -> Result<Self, String> {
        let mut runtime = Self::load_from_paths_with_latent_and_controller(
            construction_path,
            contrastive_path,
            latent_path,
            controller_path,
        )?;
        runtime.expression_inventory = Some(
            ExpressionConstructionInventoryArtifactIR::load_from_path(inventory_path)?,
        );
        Ok(runtime)
    }

    pub fn model_sha256(&self) -> &str {
        &self.model.model_sha256
    }

    pub fn realize(
        &self,
        source: &ApprovedCompositionalResponseIR,
        language: LanguageCodeIR,
        context: PersonaConstructionContextIR,
    ) -> Result<PersonaConstructionOutputIR, String> {
        if !source.validate()
            || !matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::English)
        {
            return Err("PERSONA_CONSTRUCTION_SOURCE_INVALID".into());
        }
        if !context.language_state.validate() {
            return Err("LANGUAGE_STATE_CONDITION_INVALID".into());
        }
        let mut projected = source.clone();
        let learned_profile = context
            .persona_label
            .as_deref()
            .map(|label| {
                let key = construction_key_sha256_for_response(language, &projected);
                self.contrastive_model
                    .as_ref()
                    .and_then(|model| {
                        // Known construction keys use the paired contrastive
                        // family.  An unseen composition deliberately falls
                        // back to the learned persona operator so the same
                        // construction inventory can compose productively.
                        model
                            .profile_for_label_and_construction(label, &key)
                            .or_else(|| model.profile_for_label(label))
                    })
                    .ok_or_else(|| format!("PERSONA_CONTRASTIVE_PROFILE_UNSUPPORTED:{label}:{key}"))
            })
            .transpose()?;
        if !(-1_000..=1_000).contains(&context.verbosity_delta_millis) {
            return Err("PERSONA_VERBOSITY_OPERATOR_INVALID".into());
        }
        if context.latent_operator_deltas_millis.len()
            > self
                .latent_model
                .as_ref()
                .map_or(0, PersonaLatentOperatorArtifactIR::component_count)
            || context
                .latent_operator_deltas_millis
                .iter()
                .any(|value| !(-1_000..=1_000).contains(value))
        {
            return Err("PERSONA_LATENT_OPERATOR_INTERVENTION_INVALID".into());
        }
        let language_state_projection = self
            .language_state_controller
            .as_ref()
            .map(|controller| controller.project(&language_state_values(&context.language_state)));
        let mut effective_latent_deltas = context.latent_operator_deltas_millis.clone();
        if let Some(projection) = &language_state_projection {
            if self.latent_model.is_some() {
                if effective_latent_deltas.is_empty() {
                    effective_latent_deltas = projection.latent_deltas_millis.clone();
                } else {
                    effective_latent_deltas.resize(
                        effective_latent_deltas
                            .len()
                            .max(projection.latent_deltas_millis.len()),
                        0,
                    );
                    for (index, delta) in projection.latent_deltas_millis.iter().enumerate() {
                        effective_latent_deltas[index] = effective_latent_deltas[index]
                            .saturating_add(*delta)
                            .clamp(-1_000, 1_000);
                    }
                }
            }
        }
        let latent_controls = self
            .latent_model
            .as_ref()
            .map(|model| model.controls(&effective_latent_deltas));
        projected.style = style_for(
            &context,
            learned_profile,
            latent_controls.as_ref(),
            language_state_projection
                .as_ref()
                .map_or(0, |projection| projection.verbosity_millis),
        );
        projected.semantic_sha256 = compositional_response_sha256(&projected);
        if !projected.validate()
            || !same_authoritative_meaning(source, &projected)
            || !self.model.supports_style(language, projected.style.clone())
        {
            return Err("PERSONA_CONSTRUCTION_PROFILE_UNSUPPORTED".into());
        }
        let document = realize_document_response(&projected, language)
            .map_err(|_| "PERSONA_CONSTRUCTION_DOCUMENT_FAILED")?;
        let document_markdown = if self.expression_inventory.as_ref().is_some_and(|inventory| {
            inventory.supports(
                "ENSURE_EXPLICIT_STATUS_LABEL",
                language_state_projection
                    .as_ref()
                    .map_or(0, |projection| projection.verbosity_millis),
            )
        }) {
            ensure_explicit_status_label(&document.markdown, &projected, language)
        } else {
            document.markdown.clone()
        };
        let inverse = interpret_document_semantics(&document_markdown, &projected, language)
            .map_err(|_| "PERSONA_CONSTRUCTION_INVERSE_FAILED")?;
        if !inverse.validate(&projected) {
            return Err("PERSONA_CONSTRUCTION_INVERSE_INVALID".into());
        }
        let marker = marker_for(context.mode, latent_controls.as_ref());
        let markdown = match marker {
            PersonaDiscourseMarkerIR::None => document_markdown,
            _ => format!(
                "{}\n\n{}",
                marker_surface(marker, language),
                document_markdown
            ),
        };
        let mut output = PersonaConstructionOutputIR {
            schema: PERSONA_CONSTRUCTION_OUTPUT_SCHEMA.into(),
            source_response_sha256: source.semantic_sha256.clone(),
            projected_response_sha256: projected.semantic_sha256,
            model_sha256: self.model.model_sha256.clone(),
            contrastive_model_sha256: self
                .contrastive_model
                .as_ref()
                .map(|model| model.model_sha256.clone()),
            contrastive_profile: learned_profile.map(|profile| profile.persona_label.clone()),
            context,
            language_state_projection,
            marker,
            markdown,
            semantic_inverse: inverse,
            output_sha256: String::new(),
        };
        output.output_sha256 = persona_construction_output_sha256(&output);
        output
            .validate(source)
            .then_some(output)
            .ok_or_else(|| "PERSONA_CONSTRUCTION_OUTPUT_INVALID".into())
    }
}

impl PersonaConstructionOutputIR {
    pub fn validate(&self, source: &ApprovedCompositionalResponseIR) -> bool {
        self.schema == PERSONA_CONSTRUCTION_OUTPUT_SCHEMA
            && source.validate()
            && sha256_shape(&self.source_response_sha256)
            && sha256_shape(&self.projected_response_sha256)
            && sha256_shape(&self.model_sha256)
            && self
                .contrastive_model_sha256
                .as_deref()
                .map_or(true, sha256_shape)
            && !self.markdown.trim().is_empty()
            && self.semantic_inverse.source_response_sha256 == self.projected_response_sha256
            && self.semantic_inverse.claims.len() == source.claims.len()
            && self.semantic_inverse.claims.iter().all(|recovered| {
                source.claims.iter().any(|claim| {
                    claim.proposition_id == recovered.proposition_id
                        && claim.subject == recovered.subject
                        && claim.relation == recovered.relation
                        && claim.value == recovered.value
                        && claim.polarity == recovered.polarity
                        && claim.modality == recovered.modality
                })
            })
            && self.output_sha256 == persona_construction_output_sha256(self)
    }
}

pub fn same_authoritative_meaning(
    source: &ApprovedCompositionalResponseIR,
    projected: &ApprovedCompositionalResponseIR,
) -> bool {
    source.claims == projected.claims
        && source.operation == projected.operation
        && source.speech_act == projected.speech_act
        && source.discourse_relation == projected.discourse_relation
        && source.event_realizations == projected.event_realizations
        && source.source_world_state_sha256 == projected.source_world_state_sha256
        && source.source_deliberation_sha256 == projected.source_deliberation_sha256
        && source.approval_replay_verified == projected.approval_replay_verified
        && source.unsupported_claims == projected.unsupported_claims
}

pub fn persona_construction_output_sha256(output: &PersonaConstructionOutputIR) -> String {
    let mut canonical = output.clone();
    canonical.output_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).expect("persona output serializes")),
    )
}

fn language_state_values(
    state: &LanguageStateConditionIR,
) -> std::collections::BTreeMap<String, String> {
    let mut values = std::collections::BTreeMap::new();
    if let Some(value) = &state.personality {
        values.insert("personality".into(), value.clone());
    }
    if let Some(value) = &state.speaker_emotion {
        values.insert("current_speaker_emotion".into(), value.clone());
    }
    if let Some(value) = &state.inferred_user_emotion {
        values.insert("inferred_user_emotion".into(), value.clone());
    }
    if let Some(value) = state.relationship {
        values.insert(
            "relationship_social_distance".into(),
            format!("{value:?}").to_ascii_uppercase(),
        );
    }
    if let Some(value) = state.dialogue_context {
        values.insert(
            "dialogue_context".into(),
            format!("{value:?}").to_ascii_uppercase(),
        );
    }
    if let Some(value) = state.age_band {
        values.insert(
            "age_register_condition".into(),
            format!("{value:?}").to_ascii_uppercase(),
        );
    }
    if let Some(value) = state.register {
        values.insert(
            "register_condition".into(),
            format!("{value:?}").to_ascii_uppercase(),
        );
    }
    if let Some(value) = state.korean_dialect {
        values.insert(
            "region_dialect_condition".into(),
            format!("{value:?}").to_ascii_uppercase(),
        );
    }
    values
}

/// Applies a learned, meaning-preserving structural transformation.  It only
/// makes an already realized STATUS relation explicit; it never invents a
/// claim or chooses a lexical predicate.  The inverse is run after this
/// transformation and therefore remains the authority for acceptance.
fn ensure_explicit_status_label(
    markdown: &str,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> String {
    if language != LanguageCodeIR::Korean {
        return markdown.to_string();
    }
    let mut result = markdown.to_string();
    for claim in response
        .claims
        .iter()
        .filter(|claim| claim.relation == ApprovedRelationTypeIR::Status)
    {
        if !matches!(claim.value, ApprovedOpenValueIR::Lexical(_)) {
            continue;
        }
        let label = claim.subject.canonical_lexical_label.trim();
        if label.is_empty() {
            continue;
        }
        for particle in ["는", "은"] {
            let compact = format!("{label}{particle} ");
            let explicit = format!("{label} 상태{particle} ");
            result = result.replace(&compact, &explicit);
        }
    }
    result
}

fn style_for(
    context: &PersonaConstructionContextIR,
    learned_profile: Option<&crate::persona_contrastive_training::PersonaContrastiveProfileIR>,
    latent_controls: Option<&PersonaLatentControlsIR>,
    controller_verbosity_millis: i16,
) -> ApprovedResponseStyleIR {
    let mut style = if let Some(profile) = learned_profile {
        let register = match profile.register.as_str() {
            "FORMAL" => LanguageRegisterIR::Formal,
            "INFORMAL" => LanguageRegisterIR::Informal,
            _ => LanguageRegisterIR::Neutral,
        };
        ApprovedResponseStyleIR {
            register,
            verbosity: match profile.voice.as_str() {
                "LIVELY" | "GENTLE" => ApprovedVerbosityIR::Explanatory,
                _ => ApprovedVerbosityIR::Short,
            },
        }
    } else {
        match context.mode {
            PersonaConstructionModeIR::DefaultPolite => ApprovedResponseStyleIR {
                register: LanguageRegisterIR::Neutral,
                verbosity: ApprovedVerbosityIR::Short,
            },
            PersonaConstructionModeIR::FriendlyPolite => ApprovedResponseStyleIR {
                register: LanguageRegisterIR::Neutral,
                verbosity: ApprovedVerbosityIR::Explanatory,
            },
            PersonaConstructionModeIR::Concise => ApprovedResponseStyleIR {
                register: if context.persona.relationship == RoleplayRelationshipIR::Professional {
                    LanguageRegisterIR::Formal
                } else {
                    LanguageRegisterIR::Informal
                },
                verbosity: ApprovedVerbosityIR::Short,
            },
            PersonaConstructionModeIR::Explanatory => ApprovedResponseStyleIR {
                register: LanguageRegisterIR::Formal,
                verbosity: ApprovedVerbosityIR::Explanatory,
            },
            PersonaConstructionModeIR::CalmSupportive => ApprovedResponseStyleIR {
                register: if context.persona.voice == RoleplayVoiceIR::Direct {
                    LanguageRegisterIR::Neutral
                } else {
                    LanguageRegisterIR::Formal
                },
                verbosity: ApprovedVerbosityIR::Explanatory,
            },
        }
    };
    let latent_verbosity = latent_controls.map_or(0, |controls| controls.verbosity_millis);
    let effective_verbosity = i32::from(context.verbosity_delta_millis)
        + i32::from(controller_verbosity_millis)
        + i32::from(latent_verbosity);
    if effective_verbosity <= -250 {
        style.verbosity = ApprovedVerbosityIR::Short;
    } else if effective_verbosity >= 250 {
        style.verbosity = ApprovedVerbosityIR::Explanatory;
    }
    if let Some(controls) = latent_controls {
        if controls.formality_millis >= 250 {
            style.register = LanguageRegisterIR::Formal;
        } else if controls.formality_millis <= -250 {
            style.register = LanguageRegisterIR::Informal;
        }
    }
    style
}

fn marker_for(
    mode: PersonaConstructionModeIR,
    latent_controls: Option<&PersonaLatentControlsIR>,
) -> PersonaDiscourseMarkerIR {
    if let Some(controls) = latent_controls {
        if controls.discourse_millis >= 250 {
            return PersonaDiscourseMarkerIR::FriendlyOrientation;
        }
        if controls.discourse_millis <= -250 {
            return PersonaDiscourseMarkerIR::None;
        }
    }
    match mode {
        PersonaConstructionModeIR::FriendlyPolite => PersonaDiscourseMarkerIR::FriendlyOrientation,
        PersonaConstructionModeIR::CalmSupportive => {
            PersonaDiscourseMarkerIR::SupportiveOrientation
        }
        _ => PersonaDiscourseMarkerIR::None,
    }
}

fn marker_surface(marker: PersonaDiscourseMarkerIR, language: LanguageCodeIR) -> &'static str {
    match (marker, language) {
        (PersonaDiscourseMarkerIR::FriendlyOrientation, LanguageCodeIR::Korean) => {
            "편하게 말씀드리면,"
        }
        (PersonaDiscourseMarkerIR::SupportiveOrientation, LanguageCodeIR::Korean) => {
            "확인된 내용만 차분히 안내드릴게요."
        }
        (PersonaDiscourseMarkerIR::FriendlyOrientation, _) => "To put it simply,",
        (PersonaDiscourseMarkerIR::SupportiveOrientation, _) => {
            "I will stay with the confirmed details."
        }
        (PersonaDiscourseMarkerIR::None, _) => "",
    }
}

fn sha256_shape(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approved_response::{
        compositional_response_sha256, ApprovedCompositionalClaimIR, ApprovedDiscourseRelationIR,
        ApprovedLexicalNodeIR, ApprovedModalityIR, ApprovedOpenValueIR, ApprovedOperationIR,
        ApprovedRelationTypeIR, ApprovedSemanticTypeIR, ApprovedSpeechActIR,
        COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA,
    };
    use crate::canonical_response_training::{
        CanonicalResponseCodecModelIR, CanonicalResponseTrainingCampaignIR,
    };

    fn time_revision() -> ApprovedCompositionalResponseIR {
        let mut response = ApprovedCompositionalResponseIR {
            schema: COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA.into(),
            speech_act: ApprovedSpeechActIR::Inform,
            operation: ApprovedOperationIR::Revise,
            claims: vec![ApprovedCompositionalClaimIR {
                proposition_id: "MEETING_TIME".into(),
                subject: ApprovedLexicalNodeIR {
                    node_id: "MEETING".into(),
                    semantic_type: ApprovedSemanticTypeIR::Event,
                    canonical_lexical_label: "회의".into(),
                },
                relation: ApprovedRelationTypeIR::Time,
                value: ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 0,
                },
                polarity: true,
                modality: ApprovedModalityIR::Asserted,
    status_frame: None,
}],
            event_realizations: Vec::new(),
            discourse_relation: ApprovedDiscourseRelationIR::Correction,
            clause_plan: Vec::new(),
            style: ApprovedResponseStyleIR {
                register: LanguageRegisterIR::Neutral,
                verbosity: ApprovedVerbosityIR::Short,
            },
            source_world_state_sha256: "a".repeat(64),
            source_deliberation_sha256: "b".repeat(64),
            approval_replay_verified: true,
            unsupported_claims: 0,
            semantic_sha256: String::new(),
        };
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        response
    }

    fn blind_three_clause_composition() -> ApprovedCompositionalResponseIR {
        let mut response = time_revision();
        response.operation = ApprovedOperationIR::Assert;
        response.discourse_relation = ApprovedDiscourseRelationIR::Statement;
        response.claims = ["회의", "자료", "참석 확인"]
            .into_iter()
            .enumerate()
            .map(|(index, label)| ApprovedCompositionalClaimIR {
                proposition_id: format!("BLIND_STATUS_{index}"),
                subject: ApprovedLexicalNodeIR {
                    node_id: format!("BLIND_EVENT_{index}"),
                    semantic_type: ApprovedSemanticTypeIR::Event,
                    canonical_lexical_label: label.into(),
                },
                relation: ApprovedRelationTypeIR::Status,
                value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "PLANNED".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "계획됨".into(),
                }),
                polarity: true,
                modality: ApprovedModalityIR::Asserted,
    status_frame: None,
})
            .collect();
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        response
    }

    fn runtime() -> ConstructionRuntime {
        let campaign = CanonicalResponseTrainingCampaignIR::build_korean_plan_descriptions()
            .expect("campaign");
        let model = CanonicalResponseCodecModelIR::train(&campaign).expect("trained model");
        let path = std::env::temp_dir().join(format!(
            "bcore-construction-runtime-restart-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, serde_json::to_vec(&model).expect("model serializes"))
            .expect("durable model artifact");
        ConstructionRuntime::load_from_path(&path).expect("restart-loadable construction runtime")
    }

    #[test]
    fn loaded_profiles_change_surface_but_preserve_a_time_correction() {
        let runtime = runtime();
        let source = time_revision();
        let contexts = [
            PersonaConstructionContextIR {
                mode: PersonaConstructionModeIR::DefaultPolite,
                persona: RoleplayPersonaIR::default(),
                persona_label: None,
                verbosity_delta_millis: 0,
                latent_operator_deltas_millis: Vec::new(),
                language_state: LanguageStateConditionIR::default(),
            },
            PersonaConstructionContextIR {
                mode: PersonaConstructionModeIR::FriendlyPolite,
                persona: RoleplayPersonaIR {
                    relationship: RoleplayRelationshipIR::Peer,
                    ..RoleplayPersonaIR::default()
                },
                persona_label: None,
                verbosity_delta_millis: 0,
                latent_operator_deltas_millis: Vec::new(),
                language_state: LanguageStateConditionIR::default(),
            },
            PersonaConstructionContextIR {
                mode: PersonaConstructionModeIR::Concise,
                persona: RoleplayPersonaIR {
                    relationship: RoleplayRelationshipIR::Close,
                    voice: RoleplayVoiceIR::Direct,
                    ..RoleplayPersonaIR::default()
                },
                persona_label: None,
                verbosity_delta_millis: 0,
                latent_operator_deltas_millis: Vec::new(),
                language_state: LanguageStateConditionIR::default(),
            },
            PersonaConstructionContextIR {
                mode: PersonaConstructionModeIR::Explanatory,
                persona: RoleplayPersonaIR {
                    relationship: RoleplayRelationshipIR::Professional,
                    ..RoleplayPersonaIR::default()
                },
                persona_label: None,
                verbosity_delta_millis: 0,
                latent_operator_deltas_millis: Vec::new(),
                language_state: LanguageStateConditionIR::default(),
            },
            PersonaConstructionContextIR {
                mode: PersonaConstructionModeIR::CalmSupportive,
                persona: RoleplayPersonaIR {
                    relationship: RoleplayRelationshipIR::Caregiving,
                    voice: RoleplayVoiceIR::Gentle,
                    ..RoleplayPersonaIR::default()
                },
                persona_label: None,
                verbosity_delta_millis: 0,
                latent_operator_deltas_millis: Vec::new(),
                language_state: LanguageStateConditionIR::default(),
            },
        ];
        let outputs = contexts
            .into_iter()
            .map(|context| runtime.realize(&source, LanguageCodeIR::Korean, context))
            .collect::<Result<Vec<_>, _>>()
            .expect("persona realization");
        assert!(outputs.iter().all(|output| output.validate(&source)));
        assert!(outputs.iter().all(|output| {
            output.semantic_inverse.claims[0].value
                == ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 0,
                }
        }));
        assert!(outputs.iter().all(|output| {
            output.semantic_inverse.claims[0]
                .subject
                .canonical_lexical_label
                == "회의"
                && output.semantic_inverse.claims[0].polarity
        }));
        let distinct = outputs
            .iter()
            .map(|output| output.markdown.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(distinct.len() >= 4);
    }

    #[test]
    fn unseen_three_clause_composition_reuses_loaded_style_and_round_trips() {
        let runtime = runtime();
        let source = blind_three_clause_composition();
        let output = runtime
            .realize(
                &source,
                LanguageCodeIR::Korean,
                PersonaConstructionContextIR {
                    mode: PersonaConstructionModeIR::FriendlyPolite,
                    persona: RoleplayPersonaIR::default(),
                    persona_label: None,
                    verbosity_delta_millis: 0,
                    latent_operator_deltas_millis: Vec::new(),
                    language_state: LanguageStateConditionIR::default(),
                },
            )
            .expect("blind composition realizes");
        assert!(output.validate(&source));
        assert_eq!(output.semantic_inverse.claims.len(), 3);
        assert!(output.markdown.contains("회의"));
        assert!(output.markdown.contains("자료"));
        assert!(output.markdown.contains("참석 확인"));
    }

    #[test]
    fn language_state_conditions_cross_runtime_boundary_without_changing_meaning() {
        let runtime = runtime();
        let source = time_revision();
        let language_state = LanguageStateConditionIR {
            personality: Some("ENFP".into()),
            speaker_emotion: Some("CALM".into()),
            inferred_user_emotion: Some("WORRIED".into()),
            relationship: Some(RoleplayRelationshipIR::Professional),
            dialogue_context: Some(crate::affective_field::RoleplaySceneIR::Service),
            age_band: Some(crate::affective_field::RoleplayAgeBandIR::Adult),
            register: Some(LanguageRegisterIR::Formal),
            korean_dialect: Some(crate::affective_field::KoreanDialectIR::Standard),
        };
        let output = runtime
            .realize(
                &source,
                LanguageCodeIR::Korean,
                PersonaConstructionContextIR {
                    mode: PersonaConstructionModeIR::DefaultPolite,
                    persona: RoleplayPersonaIR::default(),
                    persona_label: None,
                    verbosity_delta_millis: 0,
                    latent_operator_deltas_millis: Vec::new(),
                    language_state: language_state.clone(),
                },
            )
            .expect("state-aware realization");
        assert_eq!(output.context.language_state, language_state);
        assert!(output.validate(&source));
        assert_eq!(output.semantic_inverse.claims.len(), source.claims.len());
    }

    #[test]
    fn speech_act_coverage_canary_roundtrips_without_meaning_rewrite() {
        let runtime = runtime();
        let cases = [
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
            ),
            (
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                ApprovedOperationIR::Reassure,
            ),
        ];
        for (speech_act, discourse_relation, operation) in cases {
            let mut source = time_revision();
            source.speech_act = speech_act;
            source.discourse_relation = discourse_relation;
            source.operation = operation;
            source.semantic_sha256 = compositional_response_sha256(&source);
            assert!(source.validate());
            let output = runtime
                .realize(
                    &source,
                    LanguageCodeIR::Korean,
                    PersonaConstructionContextIR {
                        mode: PersonaConstructionModeIR::DefaultPolite,
                        persona: RoleplayPersonaIR::default(),
                        persona_label: None,
                        verbosity_delta_millis: 0,
                        latent_operator_deltas_millis: Vec::new(),
                        language_state: LanguageStateConditionIR::default(),
                    },
                )
                .unwrap_or_else(|error| {
                    panic!("speech-act coverage realization failed: {speech_act:?}: {error}")
                });
            assert!(output.validate(&source));
            assert_eq!(output.semantic_inverse.claims.len(), source.claims.len());
            assert_eq!(
                output.semantic_inverse.claims[0].value,
                source.claims[0].value
            );
        }
    }
}
