//! Contrastive persona construction learner.
//!
//! A family is the unit of supervision: one canonical meaning paired with
//! the observed persona conditions.  The learner stores only typed axes and
//! construction signatures.  It does not store a completed surface sentence
//! or use a sentence cache as a runtime language model.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    approved_response::ApprovedCompositionalResponseIR, language_knowledge::LanguageCodeIR,
    CanonicalPersonaGoldRecordIR, CanonicalResponseDatasetSplitIR,
};

pub const PERSONA_CONTRASTIVE_MODEL_SCHEMA: &str =
    "BCORE.PERSONA_CONTRASTIVE_CONSTRUCTION_MODEL.V1";
pub const PERSONA_LATENT_OPERATOR_SCHEMA: &str = "BCORE.PERSONA_LATENT_OPERATOR_DISCOVERY.V1";
pub const LANGUAGE_STATE_CONTROLLER_SCHEMA: &str = "BCORE.LANGUAGE_STATE_CONTROLLER.V1";
pub const EXPRESSION_CONSTRUCTION_INVENTORY_SCHEMA: &str =
    "BCORE.EXPRESSION_CONSTRUCTION_INVENTORY.V1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpressionConstructionRuleIR {
    pub transformation_id: String,
    pub operation: String,
    pub control_axis: String,
    pub min_control_millis: i16,
    pub meaning_preserving: bool,
    pub source_transition_count: usize,
    pub surface_cache: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpressionConstructionInventoryArtifactIR {
    pub schema: String,
    pub source_gold_sha256: String,
    pub source_feature_inventory_sha256: String,
    pub surface_authority: String,
    pub rules: Vec<ExpressionConstructionRuleIR>,
    pub runtime_promotion: String,
    pub artifact_sha256: String,
}

impl ExpressionConstructionInventoryArtifactIR {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|_| "EXPRESSION_INVENTORY_READ_FAILED")?;
        let artifact = serde_json::from_slice::<Self>(&bytes)
            .map_err(|_| "EXPRESSION_INVENTORY_JSON_INVALID")?;
        artifact
            .validate()
            .then_some(artifact)
            .ok_or_else(|| "EXPRESSION_INVENTORY_INVALID".into())
    }

    pub fn validate(&self) -> bool {
        self.schema == EXPRESSION_CONSTRUCTION_INVENTORY_SCHEMA
            && sha256_shape(&self.source_gold_sha256)
            && sha256_shape(&self.source_feature_inventory_sha256)
            && self.surface_authority.contains("NO_TEACHER_SURFACE_CACHE")
            && self.runtime_promotion == "RESEARCH_SHADOW_ONLY"
            && self.rules.iter().all(|rule| {
                !rule.transformation_id.is_empty()
                    && !rule.operation.is_empty()
                    && rule.control_axis == "verbosity_millis"
                    && (-1_000..=1_000).contains(&rule.min_control_millis)
                    && rule.meaning_preserving
                    && !rule.surface_cache
                    && rule.source_transition_count > 0
            })
            && sha256_shape(&self.artifact_sha256)
    }

    pub fn supports(&self, operation: &str, control_millis: i16) -> bool {
        self.rules
            .iter()
            .any(|rule| rule.operation == operation && control_millis >= rule.min_control_millis)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonaLatentOperatorArtifactIR {
    pub schema: String,
    pub feature_names: Vec<String>,
    pub components: Vec<Vec<f64>>,
    pub artifact_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersonaLatentControlsIR {
    pub formality_millis: i16,
    pub discourse_millis: i16,
    pub verbosity_millis: i16,
}

/// A learned mapping from typed B_Core state values to the compact language
/// state basis `[V, L1, L2, L3, L4]`.  It contains no surface text or template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageStateControllerEntryIR {
    pub factor: String,
    pub value: String,
    pub delta_millis: Vec<i16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageStateControllerArtifactIR {
    pub schema: String,
    pub basis: Vec<String>,
    #[serde(default)]
    pub basis_scales: Vec<f64>,
    pub entries: Vec<LanguageStateControllerEntryIR>,
    pub train_observation_count: usize,
    pub validation_metrics: BTreeMap<String, f64>,
    pub blind_metrics: BTreeMap<String, f64>,
    pub artifact_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageStateProjectionIR {
    pub verbosity_millis: i16,
    pub latent_deltas_millis: Vec<i16>,
}

impl LanguageStateControllerArtifactIR {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|_| "LANGUAGE_STATE_CONTROLLER_READ_FAILED")?;
        let artifact = serde_json::from_slice::<Self>(&bytes)
            .map_err(|_| "LANGUAGE_STATE_CONTROLLER_JSON_INVALID")?;
        artifact
            .validate()
            .then_some(artifact)
            .ok_or_else(|| "LANGUAGE_STATE_CONTROLLER_INVALID".into())
    }

    pub fn validate(&self) -> bool {
        self.schema == LANGUAGE_STATE_CONTROLLER_SCHEMA
            && self.basis == ["V", "L1", "L2", "L3", "L4"]
            && (self.basis_scales.is_empty()
                || (self.basis_scales.len() == self.basis.len()
                    && self
                        .basis_scales
                        .iter()
                        .all(|value| value.is_finite() && *value != 0.0)))
            && sha256_shape(&self.artifact_sha256)
            && !self.entries.is_empty()
            && self.entries.iter().all(|entry| {
                !entry.factor.is_empty()
                    && !entry.value.is_empty()
                    && entry.delta_millis.len() == self.basis.len()
                    && entry
                        .delta_millis
                        .iter()
                        .all(|value| (-1_000..=1_000).contains(value))
            })
    }

    pub fn project(&self, values: &BTreeMap<String, String>) -> LanguageStateProjectionIR {
        let mut total = vec![0_i32; self.basis.len()];
        for entry in &self.entries {
            if values
                .get(&entry.factor)
                .is_some_and(|value| value == &entry.value)
            {
                for (index, delta) in entry.delta_millis.iter().enumerate() {
                    total[index] += i32::from(*delta);
                }
            }
        }
        let values = total
            .into_iter()
            .map(|value| value.clamp(-1_000, 1_000) as i16)
            .collect::<Vec<_>>();
        LanguageStateProjectionIR {
            verbosity_millis: values.first().copied().unwrap_or(0),
            latent_deltas_millis: values.into_iter().skip(1).collect(),
        }
    }
}

impl PersonaLatentOperatorArtifactIR {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|_| "PERSONA_LATENT_ARTIFACT_READ_FAILED")?;
        let artifact = serde_json::from_slice::<Self>(&bytes)
            .map_err(|_| "PERSONA_LATENT_ARTIFACT_JSON_INVALID")?;
        artifact
            .validate()
            .then_some(artifact)
            .ok_or_else(|| "PERSONA_LATENT_ARTIFACT_INVALID".into())
    }

    pub fn validate(&self) -> bool {
        self.schema == PERSONA_LATENT_OPERATOR_SCHEMA
            && sha256_shape(&self.artifact_sha256)
            && !self.feature_names.is_empty()
            && self.components.iter().all(|component| {
                component.len() == self.feature_names.len()
                    && component.iter().all(|value| value.is_finite())
            })
            && !self.components.is_empty()
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn controls(&self, deltas: &[i16]) -> PersonaLatentControlsIR {
        let feature = |name: &str| self.feature_names.iter().position(|item| item == name);
        let formal = feature("ending_FORMAL");
        let informal = feature("ending_INFORMAL");
        let discourse = feature("discourse");
        let emotional = feature("emotionality");
        let chars = feature("chars");
        let tokens = feature("tokens");
        let mut formality = 0.0;
        let mut discourse_signal = 0.0;
        let mut verbosity = 0.0;
        for (index, delta) in deltas.iter().enumerate() {
            let Some(component) = self.components.get(index) else {
                break;
            };
            let delta = f64::from(*delta);
            if let (Some(formal), Some(informal)) = (formal, informal) {
                formality += delta * (component[formal] - component[informal]);
            }
            if let Some(position) = discourse {
                discourse_signal += delta * component[position];
            }
            if let Some(position) = emotional {
                discourse_signal += delta * component[position] * 0.5;
            }
            if let Some(position) = chars {
                verbosity += delta * component[position];
            }
            if let Some(position) = tokens {
                verbosity += delta * component[position] * 0.5;
            }
        }
        PersonaLatentControlsIR {
            formality_millis: clamp_millis(formality),
            discourse_millis: clamp_millis(discourse_signal),
            verbosity_millis: clamp_millis(verbosity),
        }
    }
}

fn clamp_millis(value: f64) -> i16 {
    value.round().clamp(-1_000.0, 1_000.0) as i16
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaContrastiveProfileIR {
    pub persona_label: String,
    pub register: String,
    pub relationship: String,
    pub voice: String,
    pub construction_key_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaContrastiveFamilyIR {
    pub meaning_family_id: String,
    pub construction_key_sha256: String,
    pub profiles: Vec<PersonaContrastiveProfileIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaContrastiveConstructionModelIR {
    pub schema: String,
    pub source_grounded_file_sha256: String,
    pub train_family_count: usize,
    pub validation_family_count: usize,
    pub blind_family_count: usize,
    pub train_families: Vec<PersonaContrastiveFamilyIR>,
    pub model_sha256: String,
}

impl PersonaContrastiveConstructionModelIR {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|_| "PERSONA_CONTRASTIVE_ARTIFACT_READ_FAILED")?;
        let model = serde_json::from_slice::<Self>(&bytes)
            .map_err(|_| "PERSONA_CONTRASTIVE_ARTIFACT_JSON_INVALID")?;
        model
            .validate()
            .then_some(model)
            .ok_or_else(|| "PERSONA_CONTRASTIVE_ARTIFACT_INVALID".into())
    }

    pub fn train_from_jsonl(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|_| "PERSONA_CONTRASTIVE_SOURCE_READ_FAILED")?;
        let mut digest = Sha256::new();
        let mut groups = BTreeMap::<String, Vec<Observation>>::new();
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|_| "PERSONA_CONTRASTIVE_SOURCE_READ_FAILED")?;
            digest.update(line.as_bytes());
            digest.update(b"\n");
            let record = serde_json::from_str::<CanonicalPersonaGoldRecordIR>(&line)
                .map_err(|_| "PERSONA_CONTRASTIVE_RECORD_INVALID")?;
            if !record.validate() {
                return Err("PERSONA_CONTRASTIVE_RECORD_INVALID".into());
            }
            let construction_key = construction_key_sha256(&record);
            let observation = Observation {
                split: record.canonical_record.split,
                profile: PersonaContrastiveProfileIR {
                    persona_label: record.persona_label,
                    register: record.persona_register,
                    relationship: record.persona_relationship,
                    voice: record.persona_voice,
                    construction_key_sha256: construction_key,
                },
            };
            groups
                .entry(record.semantic_family_id)
                .or_default()
                .push(observation);
        }
        let mut train = Vec::new();
        let mut validation_family_count = 0;
        let mut blind_family_count = 0;
        for (family_id, observations) in groups {
            let split = observations
                .first()
                .map(|observation| observation.split)
                .ok_or("PERSONA_CONTRASTIVE_EMPTY_FAMILY")?;
            if observations
                .iter()
                .any(|observation| observation.split != split)
            {
                return Err("PERSONA_CONTRASTIVE_SPLIT_LEAKAGE".into());
            }
            let profiles = complete_profiles(&observations)?;
            match split {
                CanonicalResponseDatasetSplitIR::Train => {
                    let construction_key_sha256 = profiles
                        .first()
                        .map(|profile| profile.construction_key_sha256.clone())
                        .ok_or("PERSONA_CONTRASTIVE_EMPTY_FAMILY")?;
                    if profiles
                        .iter()
                        .any(|profile| profile.construction_key_sha256 != construction_key_sha256)
                    {
                        return Err("PERSONA_CONTRASTIVE_SEMANTIC_KEY_DISAGREEMENT".into());
                    }
                    train.push(PersonaContrastiveFamilyIR {
                        meaning_family_id: family_id,
                        construction_key_sha256,
                        profiles,
                    });
                }
                CanonicalResponseDatasetSplitIR::Validation => validation_family_count += 1,
                CanonicalResponseDatasetSplitIR::Blind => blind_family_count += 1,
            }
        }
        train.sort_by(|left, right| left.meaning_family_id.cmp(&right.meaning_family_id));
        let mut model = Self {
            schema: PERSONA_CONTRASTIVE_MODEL_SCHEMA.into(),
            source_grounded_file_sha256: format!("{:x}", digest.finalize()),
            train_family_count: train.len(),
            validation_family_count,
            blind_family_count,
            train_families: train,
            model_sha256: String::new(),
        };
        model.model_sha256 = persona_contrastive_model_sha256(&model);
        model
            .validate()
            .then_some(model)
            .ok_or_else(|| "PERSONA_CONTRASTIVE_MODEL_INVALID".into())
    }

    pub fn validate(&self) -> bool {
        self.schema == PERSONA_CONTRASTIVE_MODEL_SCHEMA
            && sha256_shape(&self.source_grounded_file_sha256)
            && self.train_family_count == self.train_families.len()
            && self.validation_family_count > 0
            && self.blind_family_count > 0
            && self.train_families.iter().all(|family| {
                family.profiles.len() == 17
                    && family
                        .profiles
                        .iter()
                        .map(|profile| profile.persona_label.as_str())
                        .collect::<BTreeSet<_>>()
                        .len()
                        == 17
                    && family.profiles.iter().all(|profile| {
                        profile.construction_key_sha256 == family.construction_key_sha256
                    })
            })
            && self.model_sha256 == persona_contrastive_model_sha256(self)
    }

    /// Returns the learned profile for an explicitly selected persona label.
    /// The label is a language-realization selector only; it never changes
    /// the approved meaning or supplies world knowledge.
    pub fn profile_for_label(&self, label: &str) -> Option<&PersonaContrastiveProfileIR> {
        self.train_families.first().and_then(|family| {
            family
                .profiles
                .iter()
                .find(|profile| profile.persona_label == label)
        })
    }

    pub fn profile_for_label_and_construction(
        &self,
        label: &str,
        construction_key_sha256: &str,
    ) -> Option<&PersonaContrastiveProfileIR> {
        self.train_families
            .iter()
            .find(|family| family.construction_key_sha256 == construction_key_sha256)
            .and_then(|family| {
                family
                    .profiles
                    .iter()
                    .find(|profile| profile.persona_label == label)
            })
    }
}

#[derive(Debug, Clone)]
struct Observation {
    split: CanonicalResponseDatasetSplitIR,
    profile: PersonaContrastiveProfileIR,
}

fn complete_profiles(
    observations: &[Observation],
) -> Result<Vec<PersonaContrastiveProfileIR>, String> {
    let mut profiles = observations
        .iter()
        .map(|observation| observation.profile.clone())
        .collect::<Vec<_>>();
    profiles.sort_by(|left, right| left.persona_label.cmp(&right.persona_label));
    let labels = profiles
        .iter()
        .map(|profile| profile.persona_label.as_str())
        .collect::<BTreeSet<_>>();
    if profiles.len() != 17 || labels.len() != 17 {
        return Err("PERSONA_CONTRASTIVE_FAMILY_NOT_17_WAY".into());
    }
    Ok(profiles)
}

fn construction_key_sha256(record: &CanonicalPersonaGoldRecordIR) -> String {
    construction_key_sha256_for_response(
        record.canonical_record.language,
        &record.canonical_record.approved_response,
    )
}

pub fn construction_key_sha256_for_response(
    language: LanguageCodeIR,
    response: &ApprovedCompositionalResponseIR,
) -> String {
    let key = serde_json::json!({
        "language": language,
        "speech_act": response.speech_act,
        "operation": response.operation,
        "discourse_relation": response.discourse_relation,
        "claim_relations": response.claims.iter().map(|claim| claim.relation).collect::<Vec<_>>(),
        "event_realizations": response.event_realizations.iter().map(|event| {
            serde_json::json!({"class":event.class,"predicate_sense":event.predicate_sense,"phase":event.phase})
        }).collect::<Vec<_>>(),
    });
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&key).expect("construction key serializes"))
    )
}

pub fn persona_contrastive_model_sha256(model: &PersonaContrastiveConstructionModelIR) -> String {
    let mut canonical = model.clone();
    canonical.model_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).expect("contrastive model serializes"))
    )
}

fn sha256_shape(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
