//! Bounded training campaign for the canonical language-codec boundary.
//!
//! The campaign starts with a typed synthetic world-state seed, lets the
//! semantic planner create and verify a plan bundle, issues an approved
//! response through the replay-verified bridge, and only then derives a
//! source-free training record.  It deliberately has no request text,
//! parser state, hidden state, or old generated response in its input.

use std::collections::BTreeSet;

use dockable_semantic_core::{
    DockableCore, PlanIntentIR, SemanticPlanArgumentIR, SemanticPlanEventIR, SemanticPlanGoalIR,
    SemanticPlanProjectionIR, SemanticPlanRoleIR, SEMANTIC_PLAN_GOAL_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::approved_response::{
    ApprovedCompositionalResponseIR, ApprovedDiscourseRelationIR, ApprovedEventPhaseIR,
    ApprovedEventPredicateSenseIR, ApprovedEventRealizationClassIR, ApprovedRelationTypeIR,
    ApprovedResponseStyleIR, ApprovedSpeechActIR, ApprovedVerbosityIR,
};
use crate::canonical_response_bridge::CanonicalResponseBridge;
use crate::canonical_response_dataset::{
    CanonicalResponseDatasetSplitIR, CanonicalResponseTrainingRecordIR,
};
use crate::language_knowledge::{LanguageCodeIR, LanguageRegisterIR};

pub const CANONICAL_RESPONSE_TRAINING_CAMPAIGN_SCHEMA: &str =
    "BCORE.CANONICAL_RESPONSE_TRAINING_CAMPAIGN.V1";
pub const CANONICAL_RESPONSE_CODEC_MODEL_SCHEMA: &str = "BCORE.CANONICAL_RESPONSE_CODEC_MODEL.V1";

const CAMPAIGN_RECORD_COUNT: usize = 300;

/// Structured synthetic state only. `lexical_label` is a typed world label
/// authorized for realization; it is not a user utterance or teacher text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntheticPlanStateSeedIR {
    pub seed_id: String,
    pub semantic_family_id: String,
    pub intent: PlanIntentIR,
    pub topic_concept_id: String,
    pub lexical_label: String,
    pub style: ApprovedResponseStyleIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalResponseTrainingCampaignIR {
    pub schema: String,
    pub campaign_id: String,
    pub records: Vec<CanonicalResponseTrainingRecordIR>,
    pub campaign_sha256: String,
}

impl CanonicalResponseTrainingCampaignIR {
    pub fn build_korean_plan_descriptions() -> Result<Self, String> {
        let core =
            DockableCore::load_embedded().map_err(|_| "CANONICAL_CAMPAIGN_CORE_LOAD_FAILED")?;
        let seeds = korean_plan_state_seeds();
        if seeds.len() != CAMPAIGN_RECORD_COUNT {
            return Err("CANONICAL_CAMPAIGN_SEED_COUNT_INVALID".into());
        }
        let mut records = Vec::with_capacity(seeds.len());
        for seed in &seeds {
            let goal = goal_from_seed(seed)?;
            let bundle = core
                .generate_semantic_plan(&goal)
                .map_err(|_| "CANONICAL_CAMPAIGN_PLANNING_FAILED")?;
            let approved = CanonicalResponseBridge.issue_plan_description_with_style(
                &goal,
                &bundle,
                LanguageCodeIR::Korean,
                seed.style.clone(),
            )?;
            let split = split_for_family(&seed.semantic_family_id);
            records.push(CanonicalResponseTrainingRecordIR::from_approved_with_split(
                approved,
                LanguageCodeIR::Korean,
                split,
            )?);
        }
        let mut campaign = Self {
            schema: CANONICAL_RESPONSE_TRAINING_CAMPAIGN_SCHEMA.into(),
            campaign_id: "SYNTHETIC-WORLD-PLAN-DESCRIPTIONS-KO-V1".into(),
            records,
            campaign_sha256: String::new(),
        };
        campaign.campaign_sha256 = canonical_response_training_campaign_sha256(&campaign);
        campaign
            .validate()
            .then_some(campaign)
            .ok_or_else(|| "CANONICAL_CAMPAIGN_INVALID".into())
    }

    pub fn validate(&self) -> bool {
        let record_ids = self
            .records
            .iter()
            .map(|record| record.record_id.as_str())
            .collect::<BTreeSet<_>>();
        let semantic_ids = self
            .records
            .iter()
            .map(|record| record.approved_response.semantic_sha256.as_str())
            .collect::<BTreeSet<_>>();
        self.schema == CANONICAL_RESPONSE_TRAINING_CAMPAIGN_SCHEMA
            && self.campaign_id == "SYNTHETIC-WORLD-PLAN-DESCRIPTIONS-KO-V1"
            && self.records.len() == CAMPAIGN_RECORD_COUNT
            && record_ids.len() == self.records.len()
            && semantic_ids.len() == self.records.len()
            && self
                .records
                .iter()
                .all(CanonicalResponseTrainingRecordIR::validate)
            && [
                CanonicalResponseDatasetSplitIR::Train,
                CanonicalResponseDatasetSplitIR::Validation,
                CanonicalResponseDatasetSplitIR::Blind,
            ]
            .into_iter()
            .all(|split| self.records.iter().any(|record| record.split == split))
            && self.campaign_sha256 == canonical_response_training_campaign_sha256(self)
    }
}

/// A learned, source-free structural model.  It records only construction
/// signatures and empirical counts; it never caches or injects a completed
/// sentence.  The language codec remains responsible for morphology and the
/// semantic inverse remains the acceptance authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalResponseCodecModelIR {
    pub schema: String,
    pub source_campaign_sha256: String,
    pub train_record_sha256s: Vec<String>,
    pub constructions: Vec<CanonicalResponseConstructionIR>,
    pub model_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalResponseConstructionKeyIR {
    pub language: LanguageCodeIR,
    pub speech_act: ApprovedSpeechActIR,
    pub discourse_relation: ApprovedDiscourseRelationIR,
    pub register: LanguageRegisterIR,
    pub verbosity: ApprovedVerbosityIR,
    pub claim_relations: Vec<ApprovedRelationTypeIR>,
    pub event_classes: Vec<ApprovedEventRealizationClassIR>,
    pub predicate_senses: Vec<Option<ApprovedEventPredicateSenseIR>>,
    pub event_phases: Vec<Option<ApprovedEventPhaseIR>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalResponseConstructionIR {
    pub key: CanonicalResponseConstructionKeyIR,
    pub observation_count: u32,
}

impl CanonicalResponseCodecModelIR {
    pub fn train(campaign: &CanonicalResponseTrainingCampaignIR) -> Result<Self, String> {
        if !campaign.validate() {
            return Err("CANONICAL_CAMPAIGN_NOT_VALIDATED".into());
        }
        let train = campaign
            .records
            .iter()
            .filter(|record| record.split == CanonicalResponseDatasetSplitIR::Train)
            .collect::<Vec<_>>();
        if train.is_empty() || train.iter().any(|record| !record.validate()) {
            return Err("CANONICAL_CODEC_TRAINING_INPUT_INVALID".into());
        }
        let mut constructions = Vec::<CanonicalResponseConstructionIR>::new();
        for record in &train {
            let key = construction_key(record);
            if let Some(existing) = constructions.iter_mut().find(|entry| entry.key == key) {
                existing.observation_count += 1;
            } else {
                constructions.push(CanonicalResponseConstructionIR {
                    key,
                    observation_count: 1,
                });
            }
        }
        constructions.sort_by_key(construction_sort_key);
        let mut model = Self {
            schema: CANONICAL_RESPONSE_CODEC_MODEL_SCHEMA.into(),
            source_campaign_sha256: campaign.campaign_sha256.clone(),
            train_record_sha256s: train
                .iter()
                .map(|record| record.record_sha256.clone())
                .collect(),
            constructions,
            model_sha256: String::new(),
        };
        model.model_sha256 = canonical_response_codec_model_sha256(&model);
        model
            .validate()
            .then_some(model)
            .ok_or_else(|| "CANONICAL_CODEC_MODEL_INVALID".into())
    }

    pub fn supports(
        &self,
        response: &ApprovedCompositionalResponseIR,
        language: LanguageCodeIR,
    ) -> bool {
        self.validate()
            && response.validate()
            && self.constructions.iter().any(|construction| {
                construction.key == construction_key_from_response(response, language)
            })
    }

    /// Style availability is learned independently from open semantic values.
    /// A new entity or time must not require retraining merely to select an
    /// already-observed Korean realization profile.
    pub fn supports_style(&self, language: LanguageCodeIR, style: ApprovedResponseStyleIR) -> bool {
        self.validate()
            && self.constructions.iter().any(|construction| {
                construction.key.language == language
                    && construction.key.register == style.register
                    && construction.key.verbosity == style.verbosity
            })
    }

    pub fn validate(&self) -> bool {
        let record_ids = self
            .train_record_sha256s
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        self.schema == CANONICAL_RESPONSE_CODEC_MODEL_SCHEMA
            && sha256_shape(&self.source_campaign_sha256)
            && !self.train_record_sha256s.is_empty()
            && record_ids.len() == self.train_record_sha256s.len()
            && self
                .train_record_sha256s
                .iter()
                .all(|value| sha256_shape(value))
            && !self.constructions.is_empty()
            && self
                .constructions
                .windows(2)
                .all(|pair| construction_sort_key(&pair[0]) < construction_sort_key(&pair[1]))
            && self
                .constructions
                .iter()
                .all(|entry| entry.observation_count > 0)
            && self.model_sha256 == canonical_response_codec_model_sha256(self)
    }
}

pub fn canonical_response_training_campaign_sha256(
    campaign: &CanonicalResponseTrainingCampaignIR,
) -> String {
    let mut canonical = campaign.clone();
    canonical.campaign_sha256.clear();
    sha256_json(&canonical)
}

pub fn canonical_response_codec_model_sha256(model: &CanonicalResponseCodecModelIR) -> String {
    let mut canonical = model.clone();
    canonical.model_sha256.clear();
    sha256_json(&canonical)
}

fn construction_key(
    record: &CanonicalResponseTrainingRecordIR,
) -> CanonicalResponseConstructionKeyIR {
    construction_key_from_response(&record.approved_response, record.language)
}

fn construction_key_from_response(
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> CanonicalResponseConstructionKeyIR {
    CanonicalResponseConstructionKeyIR {
        language,
        speech_act: response.speech_act,
        discourse_relation: response.discourse_relation,
        register: response.style.register,
        verbosity: response.style.verbosity,
        claim_relations: response.claims.iter().map(|claim| claim.relation).collect(),
        event_classes: response
            .event_realizations
            .iter()
            .map(|event| event.class)
            .collect(),
        predicate_senses: response
            .event_realizations
            .iter()
            .map(|event| event.predicate_sense)
            .collect(),
        event_phases: response
            .event_realizations
            .iter()
            .map(|event| event.phase)
            .collect(),
    }
}

fn construction_sort_key(construction: &CanonicalResponseConstructionIR) -> String {
    serde_json::to_string(&construction.key).expect("construction key serializes")
}

fn korean_plan_state_seeds() -> Vec<SyntheticPlanStateSeedIR> {
    const TOPICS: &[(&str, &str)] = &[
        ("C_SYNTHETIC_INSPECTION", "정기 점검"),
        ("C_SYNTHETIC_BRIEFING", "운영 안내"),
        ("C_SYNTHETIC_BACKUP", "자료 백업"),
        ("C_SYNTHETIC_REPAIR", "장비 수리"),
        ("C_SYNTHETIC_REVIEW", "안전 검토"),
        ("C_SYNTHETIC_TRAINING", "현장 교육"),
        ("C_SYNTHETIC_DELIVERY", "물품 전달"),
        ("C_SYNTHETIC_MIGRATION", "서버 이전"),
        ("C_SYNTHETIC_CLEANUP", "보관 정리"),
        ("C_SYNTHETIC_CONFIRMATION", "일정 확인"),
    ];
    let intents = [
        PlanIntentIR::Plan,
        PlanIntentIR::Investigate,
        PlanIntentIR::Repair,
        PlanIntentIR::Create,
        PlanIntentIR::Learn,
        PlanIntentIR::Explain,
        PlanIntentIR::Communicate,
        PlanIntentIR::Execute,
        PlanIntentIR::Plan,
        PlanIntentIR::Investigate,
    ];
    (0..CAMPAIGN_RECORD_COUNT)
        .map(|index| {
            let family = index / 15;
            let (concept, topic) = TOPICS[index % TOPICS.len()];
            SyntheticPlanStateSeedIR {
                seed_id: format!("SYNTHETIC-PLAN-STATE-{index:03}"),
                semantic_family_id: format!("FAMILY-{family:02}"),
                intent: intents[index % intents.len()],
                topic_concept_id: format!("{concept}-{index:03}"),
                lexical_label: format!("{topic} {}차", index / TOPICS.len() + 1),
                style: match index % 6 {
                    0 => ApprovedResponseStyleIR {
                        register: LanguageRegisterIR::Neutral,
                        verbosity: ApprovedVerbosityIR::Short,
                    },
                    1 => ApprovedResponseStyleIR {
                        register: LanguageRegisterIR::Neutral,
                        verbosity: ApprovedVerbosityIR::Explanatory,
                    },
                    2 => ApprovedResponseStyleIR {
                        register: LanguageRegisterIR::Formal,
                        verbosity: ApprovedVerbosityIR::Short,
                    },
                    3 => ApprovedResponseStyleIR {
                        register: LanguageRegisterIR::Formal,
                        verbosity: ApprovedVerbosityIR::Explanatory,
                    },
                    4 => ApprovedResponseStyleIR {
                        register: LanguageRegisterIR::Informal,
                        verbosity: ApprovedVerbosityIR::Short,
                    },
                    _ => ApprovedResponseStyleIR {
                        register: LanguageRegisterIR::Informal,
                        verbosity: ApprovedVerbosityIR::Explanatory,
                    },
                },
            }
        })
        .collect()
}

fn goal_from_seed(seed: &SyntheticPlanStateSeedIR) -> Result<SemanticPlanGoalIR, String> {
    if seed.seed_id.is_empty()
        || seed.semantic_family_id.is_empty()
        || seed.topic_concept_id.is_empty()
        || seed.lexical_label.trim().is_empty()
    {
        return Err("SYNTHETIC_PLAN_STATE_INVALID".into());
    }
    let argument_id = format!("ARG-{}", seed.seed_id);
    let event_id = format!("EVENT-{}", seed.seed_id);
    let mut goal = SemanticPlanGoalIR {
        schema: SEMANTIC_PLAN_GOAL_SCHEMA.into(),
        goal_id: format!("GOAL-{}", seed.seed_id),
        events: vec![SemanticPlanEventIR {
            event_id: event_id.clone(),
            predicate_concept_id: format!("PREDICATE-{:?}", seed.intent),
            intent: seed.intent,
            argument_ids: vec![argument_id.clone()],
            goal_subject_argument_ids: vec![argument_id.clone()],
            projection: SemanticPlanProjectionIR::LiveRequest,
            user_request_present: true,
            external_execution_authorized: false,
        }],
        arguments: vec![SemanticPlanArgumentIR {
            argument_id,
            role: SemanticPlanRoleIR::Theme,
            concept_ids: vec![seed.topic_concept_id.clone()],
            grounded_label: seed.lexical_label.clone(),
        }],
        relations: Vec::new(),
        selected_live_event_ids: vec![event_id],
        context_semantic_ids: vec![
            seed.semantic_family_id.clone(),
            seed.topic_concept_id.clone(),
        ],
        source_semantic_sha256: sha256_json(seed),
        max_steps_per_event: 16,
        semantic_authority: false,
        language_can_execute: false,
        semantic_sha256: String::new(),
    };
    goal.seal();
    goal.validate()
        .then_some(goal)
        .ok_or_else(|| "SYNTHETIC_PLAN_GOAL_INVALID".into())
}

fn split_for_family(family: &str) -> CanonicalResponseDatasetSplitIR {
    let family_index = family
        .strip_prefix("FAMILY-")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    match family_index % 20 {
        0 => CanonicalResponseDatasetSplitIR::Blind,
        1 => CanonicalResponseDatasetSplitIR::Validation,
        _ => CanonicalResponseDatasetSplitIR::Train,
    }
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("canonical campaign serializes")),
    )
}

fn sha256_shape(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_runs_through_planner_and_trains_without_surface_as_input() {
        let campaign = CanonicalResponseTrainingCampaignIR::build_korean_plan_descriptions()
            .expect("canonical training campaign");
        assert!(campaign.validate());
        assert_eq!(campaign.records.len(), 300);
        assert!(campaign.records.iter().all(|record| {
            record.approved_response.source_world_state_sha256 != record.document_output_sha256
                && !record.surface_reference.contains("C_SYNTHETIC")
        }));
        let model = CanonicalResponseCodecModelIR::train(&campaign).expect("trained codec model");
        assert!(model.validate());
        assert!(model.supports(
            &campaign
                .records
                .iter()
                .find(|record| record.split == CanonicalResponseDatasetSplitIR::Train)
                .expect("train record")
                .approved_response,
            LanguageCodeIR::Korean,
        ));
        assert!(!model.train_record_sha256s.iter().any(|hash| {
            campaign.records.iter().any(|record| {
                record.split == CanonicalResponseDatasetSplitIR::Blind
                    && &record.record_sha256 == hash
            })
        }));
    }
}
