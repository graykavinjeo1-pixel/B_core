use semantic_core_adapters::{
    ActionExecutionStatusIR, ClaimEpistemicStatusIR, ConversationInputModalityIR,
    ConversationTurnDispositionIR, ConversationTurnRequestIR, ConversationTurnResponseIR,
    DiscourseBindingKindIR, DiscourseTopicIR, LanguageCodeIR, NativeEventScopeIR,
    NativeReferenceKindIR, NativeResponseGoalIR, NativeResponseModeIR, NaturalResponseActIR,
    UtteranceAlternativeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const REFERENCE_SUITE_SCHEMA: &str = "B_CORE_GPT_REFERENCE_SUITE_IR_1";
pub const INPUT_SUITE_SCHEMA: &str = "B_CORE_GPT_REFERENCE_INPUT_SUITE_IR_1";
pub const REFERENCE_SURFACE_RUN_SCHEMA: &str = "B_CORE_GPT_REFERENCE_SURFACE_RUN_IR_1";
pub const CANDIDATE_BATCH_SCHEMA: &str = "B_CORE_GPT_REFERENCE_CANDIDATE_BATCH_IR_1";
pub const B_CORE_RESPONSE_BATCH_SCHEMA: &str = "B_CORE_GPT_REFERENCE_RESPONSE_BATCH_IR_1";
pub const EVALUATION_REPORT_SCHEMA: &str = "B_CORE_GPT_REFERENCE_EVALUATION_REPORT_IR_1";
pub const SCORE_SCALE: u16 = 10_000;
pub const DEVELOPMENT_RESPONSE_COUNT: usize = 240;
pub const FINAL_RESPONSE_COUNT: usize = 160;
pub const DEVELOPMENT_DIALOGUE_COUNT: usize = 60;
pub const FINAL_DIALOGUE_COUNT: usize = 40;
pub const TURNS_PER_DIALOGUE: usize = 4;
pub const CALIBRATED_REFERENCE_SURFACE_COUNT: usize = 3;
pub const FINAL_RELATIVE_SURFACE_MEAN_MIN_BP: u16 = 8_500;
pub const FINAL_RELATIVE_SURFACE_P10_MIN_BP: u16 = 7_000;

pub const CATEGORIES: [&str; 10] = [
    "EXPLICIT_REQUEST",
    "INDIRECT_PRAGMATIC_INTENT",
    "CORRECTION_REJECTION_FEEDBACK",
    "DEIXIS_REFERENCE_ELLIPSIS",
    "TOPIC_SHIFT_AND_RETURN",
    "CONFLICT_UNCERTAINTY_ATTRIBUTION",
    "PLAN_EXECUTION_RESULT_BOUNDARY",
    "AFFECT_AND_SOCIAL_BACKCHANNEL",
    "AMBIGUITY_CLARIFICATION",
    "MIXED_LANGUAGE_NOISE_AND_FILLERS",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SuiteSplitIR {
    Development,
    Final,
}

impl SuiteSplitIR {
    fn expected_dialogues(self) -> usize {
        match self {
            Self::Development => DEVELOPMENT_DIALOGUE_COUNT,
            Self::Final => FINAL_DIALOGUE_COUNT,
        }
    }

    fn expected_responses(self) -> usize {
        match self {
            Self::Development => DEVELOPMENT_RESPONSE_COUNT,
            Self::Final => FINAL_RESPONSE_COUNT,
        }
    }

    fn dialogues_per_category(self) -> usize {
        self.expected_dialogues() / CATEGORIES.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvaluationLanguageIR {
    Korean,
    English,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceSurfaceVariantIR {
    pub generation_run_id: String,
    pub surface: String,
    pub surface_sha256: String,
}

impl ReferenceSurfaceVariantIR {
    pub fn new(generation_run_id: impl Into<String>, surface: impl Into<String>) -> Self {
        let surface = surface.into();
        Self {
            generation_run_id: generation_run_id.into(),
            surface_sha256: sha256_text(&surface),
            surface,
        }
    }

    fn validate(&self) -> bool {
        !self.generation_run_id.trim().is_empty()
            && !self.surface.trim().is_empty()
            && is_sha256(&self.surface_sha256)
            && self.surface_sha256 == sha256_text(&self.surface)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceTurnAnnotationIR {
    pub response_id: String,
    pub dialogue_id: String,
    pub turn_index: u8,
    pub category: String,
    pub language: EvaluationLanguageIR,
    pub response_act: String,
    pub response_goal: String,
    pub epistemic_status: String,
    pub meaning_atoms: Vec<String>,
    pub discourse_bindings: Vec<String>,
    pub required_propositions: Vec<String>,
    #[serde(default)]
    pub prohibited_propositions: Vec<String>,
    pub reference_surface: String,
    pub raw_reference_sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calibrated_reference_surfaces: Vec<ReferenceSurfaceVariantIR>,
    #[serde(default)]
    pub critical_boundary: bool,
    #[serde(default)]
    pub ambiguity_requires_clarification: bool,
}

impl ReferenceTurnAnnotationIR {
    fn scoring_surfaces(&self) -> Vec<&str> {
        if self.calibrated_reference_surfaces.is_empty() {
            vec![self.reference_surface.as_str()]
        } else {
            self.calibrated_reference_surfaces
                .iter()
                .map(|variant| variant.surface.as_str())
                .collect()
        }
    }

    fn calibrated_surface_scores(&self, candidate_surface: &str) -> Option<(u16, u16, u16)> {
        if self.calibrated_reference_surfaces.is_empty() {
            return None;
        }
        let surfaces = self.scoring_surfaces();
        let candidate_best = surfaces
            .iter()
            .map(|surface| surface_similarity_bp(surface, candidate_surface))
            .max()
            .unwrap_or_default();
        let mut pairwise = Vec::new();
        for left in 0..surfaces.len() {
            for right in left + 1..surfaces.len() {
                pairwise.push(surface_similarity_bp(surfaces[left], surfaces[right]));
            }
        }
        let self_similarity = percentile_bp(&pairwise, 50);
        let relative = ratio_bp(candidate_best as usize, self_similarity as usize);
        Some((candidate_best, self_similarity, relative))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceSuiteIR {
    pub schema: String,
    pub suite_id: String,
    pub split: SuiteSplitIR,
    pub frozen: bool,
    pub reference_model_id: String,
    pub reference_generation_date: String,
    pub reference_system_prompt_sha256: String,
    pub generation_configuration_sha256: String,
    pub input_suite_sha256: String,
    pub responses: Vec<ReferenceTurnAnnotationIR>,
    pub suite_payload_sha256: String,
}

impl ReferenceSuiteIR {
    pub fn seal(&mut self) -> Result<(), String> {
        self.suite_payload_sha256.clear();
        self.suite_payload_sha256 = content_sha256(self)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != REFERENCE_SUITE_SCHEMA {
            return Err("REFERENCE_SCHEMA_MISMATCH".to_string());
        }
        if !self.frozen {
            return Err("REFERENCE_SUITE_NOT_FROZEN".to_string());
        }
        if self.suite_id.trim().is_empty()
            || self.reference_model_id.trim().is_empty()
            || self.reference_generation_date.trim().is_empty()
        {
            return Err("REFERENCE_PROVENANCE_INCOMPLETE".to_string());
        }
        for hash in [
            &self.reference_system_prompt_sha256,
            &self.generation_configuration_sha256,
            &self.input_suite_sha256,
            &self.suite_payload_sha256,
        ] {
            if !is_sha256(hash) {
                return Err("REFERENCE_HASH_INVALID".to_string());
            }
        }
        if self.responses.len() != self.split.expected_responses() {
            return Err(format!(
                "REFERENCE_RESPONSE_DENOMINATOR_MISMATCH:{}:{}",
                self.responses.len(),
                self.split.expected_responses()
            ));
        }
        let mut response_ids = BTreeSet::new();
        let mut dialogues: BTreeMap<&str, Vec<&ReferenceTurnAnnotationIR>> = BTreeMap::new();
        let allowed_categories = CATEGORIES.into_iter().collect::<BTreeSet<_>>();
        let mut calibrated_response_count = 0_usize;
        let mut calibration_run_ids: Option<BTreeSet<String>> = None;
        for response in &self.responses {
            if !response_ids.insert(response.response_id.as_str()) {
                return Err(format!(
                    "DUPLICATE_REFERENCE_RESPONSE_ID:{}",
                    response.response_id
                ));
            }
            if response.response_id.trim().is_empty()
                || response.dialogue_id.trim().is_empty()
                || !(1..=TURNS_PER_DIALOGUE as u8).contains(&response.turn_index)
                || !allowed_categories.contains(response.category.as_str())
                || response.response_act.trim().is_empty()
                || response.response_goal.trim().is_empty()
                || response.epistemic_status.trim().is_empty()
                || response.meaning_atoms.is_empty()
                || response.required_propositions.is_empty()
                || response.reference_surface.trim().is_empty()
            {
                return Err(format!(
                    "REFERENCE_RESPONSE_INCOMPLETE:{}",
                    response.response_id
                ));
            }
            if sha256_text(&response.reference_surface) != response.raw_reference_sha256 {
                return Err(format!(
                    "REFERENCE_SURFACE_HASH_MISMATCH:{}",
                    response.response_id
                ));
            }
            if response.calibrated_reference_surfaces.is_empty() {
                if self.split == SuiteSplitIR::Final {
                    return Err(format!(
                        "FINAL_CALIBRATED_REFERENCE_SURFACES_MISSING:{}",
                        response.response_id
                    ));
                }
            } else {
                calibrated_response_count += 1;
                if response.calibrated_reference_surfaces.len()
                    != CALIBRATED_REFERENCE_SURFACE_COUNT
                {
                    return Err(format!(
                        "CALIBRATED_REFERENCE_SURFACE_COUNT_INVALID:{}:{}",
                        response.response_id,
                        response.calibrated_reference_surfaces.len()
                    ));
                }
                if response
                    .calibrated_reference_surfaces
                    .iter()
                    .any(|variant| !variant.validate())
                {
                    return Err(format!(
                        "CALIBRATED_REFERENCE_SURFACE_INVALID:{}",
                        response.response_id
                    ));
                }
                let run_ids = response
                    .calibrated_reference_surfaces
                    .iter()
                    .map(|variant| variant.generation_run_id.clone())
                    .collect::<BTreeSet<_>>();
                if run_ids.len() != CALIBRATED_REFERENCE_SURFACE_COUNT {
                    return Err(format!(
                        "CALIBRATED_REFERENCE_RUNS_NOT_INDEPENDENT:{}",
                        response.response_id
                    ));
                }
                if calibration_run_ids
                    .as_ref()
                    .is_some_and(|expected| expected != &run_ids)
                {
                    return Err(format!(
                        "CALIBRATED_REFERENCE_RUN_SET_INCONSISTENT:{}",
                        response.response_id
                    ));
                }
                calibration_run_ids.get_or_insert(run_ids);
                if !response
                    .calibrated_reference_surfaces
                    .iter()
                    .any(|variant| {
                        variant.surface == response.reference_surface
                            && variant.surface_sha256 == response.raw_reference_sha256
                    })
                {
                    return Err(format!(
                        "PRIMARY_REFERENCE_NOT_IN_CALIBRATED_SET:{}",
                        response.response_id
                    ));
                }
                if response
                    .calibrated_surface_scores(&response.reference_surface)
                    .is_none_or(|(_, self_similarity, _)| self_similarity == 0)
                {
                    return Err(format!(
                        "GPT_SELF_SURFACE_SIMILARITY_UNDEFINED:{}",
                        response.response_id
                    ));
                }
            }
            dialogues
                .entry(response.dialogue_id.as_str())
                .or_default()
                .push(response);
        }
        if calibrated_response_count != 0 && calibrated_response_count != self.responses.len() {
            return Err("PARTIAL_CALIBRATED_REFERENCE_SUITE_NOT_ALLOWED".to_string());
        }
        if dialogues.len() != self.split.expected_dialogues() {
            return Err(format!(
                "REFERENCE_DIALOGUE_DENOMINATOR_MISMATCH:{}:{}",
                dialogues.len(),
                self.split.expected_dialogues()
            ));
        }
        let mut category_counts = BTreeMap::<&str, usize>::new();
        let mut language_counts = BTreeMap::<EvaluationLanguageIR, usize>::new();
        for (dialogue_id, mut turns) in dialogues {
            turns.sort_by_key(|turn| turn.turn_index);
            if turns.len() != TURNS_PER_DIALOGUE
                || turns
                    .iter()
                    .enumerate()
                    .any(|(index, turn)| turn.turn_index as usize != index + 1)
                || turns.iter().any(|turn| {
                    turn.category != turns[0].category || turn.language != turns[0].language
                })
            {
                return Err(format!("REFERENCE_DIALOGUE_SHAPE_INVALID:{dialogue_id}"));
            }
            *category_counts
                .entry(turns[0].category.as_str())
                .or_default() += 1;
            *language_counts.entry(turns[0].language).or_default() += 1;
        }
        if CATEGORIES.iter().any(|category| {
            category_counts.get(category).copied().unwrap_or_default()
                != self.split.dialogues_per_category()
        }) {
            return Err("REFERENCE_CATEGORY_DISTRIBUTION_INVALID".to_string());
        }
        let expected_per_language = self.split.expected_dialogues() / 2;
        if language_counts
            .values()
            .any(|count| *count != expected_per_language)
            || language_counts.len() != 2
        {
            return Err("REFERENCE_LANGUAGE_DISTRIBUTION_INVALID".to_string());
        }
        let mut unhashed = self.clone();
        let expected_hash = unhashed.suite_payload_sha256.clone();
        unhashed.suite_payload_sha256.clear();
        if content_sha256(&unhashed)? != expected_hash {
            return Err("REFERENCE_SUITE_PAYLOAD_TAMPERED".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkInputTurnIR {
    pub response_id: String,
    pub dialogue_id: String,
    pub turn_index: u8,
    pub category: String,
    pub language: EvaluationLanguageIR,
    pub modality: ConversationInputModalityIR,
    pub raw_text: String,
    pub input_confidence_millis: u16,
    #[serde(default)]
    pub alternatives: Vec<UtteranceAlternativeIR>,
    #[serde(default)]
    pub context_tags: Vec<String>,
    pub max_plan_steps: usize,
}

impl BenchmarkInputTurnIR {
    pub fn to_request(&self) -> ConversationTurnRequestIR {
        ConversationTurnRequestIR {
            schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
            conversation_id: self.dialogue_id.clone(),
            turn_index: u64::from(self.turn_index),
            request_id: self.response_id.clone(),
            modality: self.modality,
            raw_text: self.raw_text.clone(),
            input_confidence_millis: self.input_confidence_millis,
            alternatives: self.alternatives.clone(),
            output_language: Some(match self.language {
                EvaluationLanguageIR::Korean => LanguageCodeIR::Korean,
                EvaluationLanguageIR::English => LanguageCodeIR::English,
            }),
            context_tags: self.context_tags.clone(),
            max_plan_steps: self.max_plan_steps,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkInputSuiteIR {
    pub schema: String,
    pub suite_id: String,
    pub split: SuiteSplitIR,
    pub frozen: bool,
    pub turns: Vec<BenchmarkInputTurnIR>,
    pub suite_payload_sha256: String,
}

impl BenchmarkInputSuiteIR {
    pub fn seal(&mut self) -> Result<(), String> {
        self.suite_payload_sha256.clear();
        self.suite_payload_sha256 = content_sha256(self)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != INPUT_SUITE_SCHEMA {
            return Err("INPUT_SUITE_SCHEMA_MISMATCH".to_string());
        }
        if !self.frozen || self.suite_id.trim().is_empty() {
            return Err("INPUT_SUITE_NOT_FROZEN_OR_UNNAMED".to_string());
        }
        if self.turns.len() != self.split.expected_responses() {
            return Err(format!(
                "INPUT_RESPONSE_DENOMINATOR_MISMATCH:{}:{}",
                self.turns.len(),
                self.split.expected_responses()
            ));
        }
        if !is_sha256(&self.suite_payload_sha256) {
            return Err("INPUT_SUITE_HASH_INVALID".to_string());
        }
        let allowed_categories = CATEGORIES.into_iter().collect::<BTreeSet<_>>();
        let mut response_ids = BTreeSet::new();
        let mut dialogues = BTreeMap::<&str, Vec<&BenchmarkInputTurnIR>>::new();
        for turn in &self.turns {
            if !response_ids.insert(turn.response_id.as_str()) {
                return Err(format!("DUPLICATE_INPUT_RESPONSE_ID:{}", turn.response_id));
            }
            if turn.response_id.trim().is_empty()
                || turn.dialogue_id.trim().is_empty()
                || !(1..=TURNS_PER_DIALOGUE as u8).contains(&turn.turn_index)
                || !allowed_categories.contains(turn.category.as_str())
                || turn.raw_text.trim().is_empty()
                || turn.input_confidence_millis > 1_000
                || turn.max_plan_steps == 0
            {
                return Err(format!("INPUT_TURN_INVALID:{}", turn.response_id));
            }
            if turn.alternatives.iter().any(|alternative| {
                alternative.text.trim().is_empty() || alternative.confidence_millis > 1_000
            }) || turn.context_tags.iter().any(|tag| tag.trim().is_empty())
            {
                return Err(format!("INPUT_TURN_METADATA_INVALID:{}", turn.response_id));
            }
            dialogues.entry(&turn.dialogue_id).or_default().push(turn);
        }
        if dialogues.len() != self.split.expected_dialogues() {
            return Err(format!(
                "INPUT_DIALOGUE_DENOMINATOR_MISMATCH:{}:{}",
                dialogues.len(),
                self.split.expected_dialogues()
            ));
        }
        let mut category_counts = BTreeMap::<&str, usize>::new();
        let mut language_counts = BTreeMap::<EvaluationLanguageIR, usize>::new();
        for (dialogue_id, mut turns) in dialogues {
            turns.sort_by_key(|turn| turn.turn_index);
            if turns.len() != TURNS_PER_DIALOGUE
                || turns
                    .iter()
                    .enumerate()
                    .any(|(index, turn)| turn.turn_index as usize != index + 1)
                || turns.iter().any(|turn| {
                    turn.category != turns[0].category || turn.language != turns[0].language
                })
            {
                return Err(format!("INPUT_DIALOGUE_SHAPE_INVALID:{dialogue_id}"));
            }
            *category_counts
                .entry(turns[0].category.as_str())
                .or_default() += 1;
            *language_counts.entry(turns[0].language).or_default() += 1;
        }
        if CATEGORIES.iter().any(|category| {
            category_counts.get(category).copied().unwrap_or_default()
                != self.split.dialogues_per_category()
        }) {
            return Err("INPUT_CATEGORY_DISTRIBUTION_INVALID".to_string());
        }
        let expected_per_language = self.split.expected_dialogues() / 2;
        if language_counts.len() != 2
            || language_counts
                .values()
                .any(|count| *count != expected_per_language)
        {
            return Err("INPUT_LANGUAGE_DISTRIBUTION_INVALID".to_string());
        }
        let mut unhashed = self.clone();
        let expected_hash = unhashed.suite_payload_sha256.clone();
        unhashed.suite_payload_sha256.clear();
        if content_sha256(&unhashed)? != expected_hash {
            return Err("INPUT_SUITE_PAYLOAD_TAMPERED".to_string());
        }
        Ok(())
    }

    pub fn validate_against_references(&self, references: &ReferenceSuiteIR) -> Result<(), String> {
        self.validate()?;
        references.validate()?;
        if self.suite_id != references.suite_id
            || self.split != references.split
            || self.suite_payload_sha256 != references.input_suite_sha256
        {
            return Err("INPUT_REFERENCE_SUITE_BINDING_MISMATCH".to_string());
        }
        let reference_shape = references
            .responses
            .iter()
            .map(|turn| {
                (
                    turn.response_id.as_str(),
                    turn.dialogue_id.as_str(),
                    turn.turn_index,
                    turn.category.as_str(),
                    turn.language,
                )
            })
            .collect::<BTreeSet<_>>();
        let input_shape = self
            .turns
            .iter()
            .map(|turn| {
                (
                    turn.response_id.as_str(),
                    turn.dialogue_id.as_str(),
                    turn.turn_index,
                    turn.category.as_str(),
                    turn.language,
                )
            })
            .collect::<BTreeSet<_>>();
        if input_shape != reference_shape {
            return Err("INPUT_REFERENCE_TURN_SHAPE_MISMATCH".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceSurfaceResponseIR {
    pub response_id: String,
    pub surface: String,
    pub surface_sha256: String,
}

impl ReferenceSurfaceResponseIR {
    pub fn new(response_id: impl Into<String>, surface: impl Into<String>) -> Self {
        let surface = surface.into();
        Self {
            response_id: response_id.into(),
            surface_sha256: sha256_text(&surface),
            surface,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceSurfaceRunIR {
    pub schema: String,
    pub suite_id: String,
    pub input_suite_sha256: String,
    pub generation_run_id: String,
    pub model_id: String,
    pub generation_date: String,
    pub system_prompt_sha256: String,
    pub generation_configuration_sha256: String,
    pub b_core_output_consulted: bool,
    pub responses: Vec<ReferenceSurfaceResponseIR>,
    pub run_payload_sha256: String,
}

impl ReferenceSurfaceRunIR {
    pub fn seal(&mut self) -> Result<(), String> {
        self.run_payload_sha256.clear();
        self.run_payload_sha256 = content_sha256(self)?;
        Ok(())
    }

    pub fn validate_against_input(&self, input: &BenchmarkInputSuiteIR) -> Result<(), String> {
        input.validate()?;
        if input.split != SuiteSplitIR::Final {
            return Err("REFERENCE_SURFACE_RUN_REQUIRES_FINAL_INPUT".to_string());
        }
        if self.schema != REFERENCE_SURFACE_RUN_SCHEMA
            || self.suite_id != input.suite_id
            || self.input_suite_sha256 != input.suite_payload_sha256
        {
            return Err("REFERENCE_SURFACE_RUN_INPUT_BINDING_INVALID".to_string());
        }
        if self.generation_run_id.trim().is_empty()
            || self.model_id.trim().is_empty()
            || self.generation_date.trim().is_empty()
            || !is_sha256(&self.system_prompt_sha256)
            || !is_sha256(&self.generation_configuration_sha256)
            || !is_sha256(&self.run_payload_sha256)
        {
            return Err("REFERENCE_SURFACE_RUN_PROVENANCE_INVALID".to_string());
        }
        if self.b_core_output_consulted {
            return Err("REFERENCE_SURFACE_RUN_B_CORE_CONTAMINATED".to_string());
        }
        if self.responses.len() != input.turns.len() {
            return Err(format!(
                "REFERENCE_SURFACE_RUN_DENOMINATOR_MISMATCH:{}:{}",
                self.responses.len(),
                input.turns.len()
            ));
        }
        let expected_ids = input
            .turns
            .iter()
            .map(|turn| turn.response_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut observed_ids = BTreeSet::new();
        for response in &self.responses {
            if !observed_ids.insert(response.response_id.as_str()) {
                return Err(format!(
                    "DUPLICATE_REFERENCE_SURFACE_RESPONSE_ID:{}",
                    response.response_id
                ));
            }
            if response.response_id.trim().is_empty()
                || response.surface.trim().is_empty()
                || !is_sha256(&response.surface_sha256)
                || response.surface_sha256 != sha256_text(&response.surface)
            {
                return Err(format!(
                    "REFERENCE_SURFACE_RESPONSE_INVALID:{}",
                    response.response_id
                ));
            }
        }
        if observed_ids != expected_ids {
            return Err("REFERENCE_SURFACE_RESPONSE_IDS_MISMATCH".to_string());
        }
        let mut unhashed = self.clone();
        let expected_hash = unhashed.run_payload_sha256.clone();
        unhashed.run_payload_sha256.clear();
        if content_sha256(&unhashed)? != expected_hash {
            return Err("REFERENCE_SURFACE_RUN_PAYLOAD_TAMPERED".to_string());
        }
        Ok(())
    }
}

pub fn seal_final_reference_suite(
    input: &BenchmarkInputSuiteIR,
    draft: &ReferenceSuiteIR,
    runs: &[ReferenceSurfaceRunIR],
) -> Result<ReferenceSuiteIR, String> {
    validate_final_reference_draft(input, draft)?;
    if runs.len() != CALIBRATED_REFERENCE_SURFACE_COUNT {
        return Err(format!(
            "FINAL_REFERENCE_RUN_COUNT_INVALID:{}:{}",
            runs.len(),
            CALIBRATED_REFERENCE_SURFACE_COUNT
        ));
    }
    for run in runs {
        run.validate_against_input(input)?;
    }
    let run_ids = runs
        .iter()
        .map(|run| run.generation_run_id.as_str())
        .collect::<BTreeSet<_>>();
    if run_ids.len() != CALIBRATED_REFERENCE_SURFACE_COUNT {
        return Err("FINAL_REFERENCE_RUN_IDS_NOT_INDEPENDENT".to_string());
    }
    let provenance = &runs[0];
    if runs.iter().skip(1).any(|run| {
        run.model_id != provenance.model_id
            || run.generation_date != provenance.generation_date
            || run.system_prompt_sha256 != provenance.system_prompt_sha256
            || run.generation_configuration_sha256 != provenance.generation_configuration_sha256
    }) {
        return Err("FINAL_REFERENCE_RUN_CONFIGURATION_MISMATCH".to_string());
    }
    let run_maps = runs
        .iter()
        .map(|run| {
            run.responses
                .iter()
                .map(|response| (response.response_id.as_str(), response))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let mut sealed = draft.clone();
    sealed.frozen = true;
    sealed.reference_model_id = provenance.model_id.clone();
    sealed.reference_generation_date = provenance.generation_date.clone();
    sealed.reference_system_prompt_sha256 = provenance.system_prompt_sha256.clone();
    sealed.generation_configuration_sha256 = provenance.generation_configuration_sha256.clone();
    for response in &mut sealed.responses {
        let variants = runs
            .iter()
            .zip(&run_maps)
            .map(|(run, responses)| {
                let surface = responses
                    .get(response.response_id.as_str())
                    .ok_or_else(|| {
                        format!("FINAL_REFERENCE_SURFACE_MISSING:{}", response.response_id)
                    })?;
                Ok(ReferenceSurfaceVariantIR::new(
                    run.generation_run_id.clone(),
                    surface.surface.clone(),
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        response.reference_surface = variants[0].surface.clone();
        response.raw_reference_sha256 = variants[0].surface_sha256.clone();
        response.calibrated_reference_surfaces = variants;
    }
    sealed.seal()?;
    sealed.validate()?;
    input.validate_against_references(&sealed)?;
    Ok(sealed)
}

pub fn validate_final_reference_draft(
    input: &BenchmarkInputSuiteIR,
    draft: &ReferenceSuiteIR,
) -> Result<(), String> {
    input.validate()?;
    if input.split != SuiteSplitIR::Final {
        return Err("FINAL_REFERENCE_DRAFT_REQUIRES_FINAL_INPUT".to_string());
    }
    if draft.schema != REFERENCE_SUITE_SCHEMA
        || draft.suite_id != input.suite_id
        || draft.split != SuiteSplitIR::Final
        || draft.frozen
        || !draft.suite_payload_sha256.is_empty()
        || draft.input_suite_sha256 != input.suite_payload_sha256
        || draft.reference_model_id.trim().is_empty()
        || draft.reference_generation_date.trim().is_empty()
        || !is_sha256(&draft.reference_system_prompt_sha256)
        || !is_sha256(&draft.generation_configuration_sha256)
        || draft.responses.len() != input.turns.len()
    {
        return Err("FINAL_REFERENCE_DRAFT_STATE_INVALID".to_string());
    }
    let input_shape = input
        .turns
        .iter()
        .map(|turn| {
            (
                turn.response_id.as_str(),
                turn.dialogue_id.as_str(),
                turn.turn_index,
                turn.category.as_str(),
                turn.language,
            )
        })
        .collect::<BTreeSet<_>>();
    let mut response_ids = BTreeSet::new();
    let mut draft_shape = BTreeSet::new();
    for response in &draft.responses {
        if !response_ids.insert(response.response_id.as_str()) {
            return Err(format!(
                "DUPLICATE_FINAL_DRAFT_RESPONSE_ID:{}",
                response.response_id
            ));
        }
        if response.response_act.trim().is_empty()
            || response.response_goal.trim().is_empty()
            || response.epistemic_status.trim().is_empty()
            || response.meaning_atoms.is_empty()
            || response.required_propositions.is_empty()
            || response.reference_surface.trim().is_empty()
            || response.raw_reference_sha256 != sha256_text(&response.reference_surface)
            || !response.calibrated_reference_surfaces.is_empty()
        {
            return Err(format!(
                "FINAL_REFERENCE_DRAFT_RESPONSE_INVALID:{}",
                response.response_id
            ));
        }
        draft_shape.insert((
            response.response_id.as_str(),
            response.dialogue_id.as_str(),
            response.turn_index,
            response.category.as_str(),
            response.language,
        ));
    }
    if draft_shape != input_shape {
        return Err("FINAL_REFERENCE_DRAFT_INPUT_SHAPE_MISMATCH".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateTurnAnnotationIR {
    pub response_id: String,
    pub response_act: String,
    pub response_goal: String,
    pub epistemic_status: String,
    pub meaning_atoms: Vec<String>,
    #[serde(default)]
    pub discourse_bindings: Vec<String>,
    pub propositions: Vec<String>,
    #[serde(default)]
    pub unsupported_propositions: Vec<String>,
    pub candidate_surface: String,
    pub candidate_surface_sha256: String,
    #[serde(default)]
    pub semantic_authority: bool,
    #[serde(default)]
    pub external_execution_authorized: bool,
    #[serde(default)]
    pub false_execution_or_result_claim: bool,
    #[serde(default)]
    pub silent_ambiguity_guess: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateBatchIR {
    pub schema: String,
    pub suite_id: String,
    pub responses: Vec<CandidateTurnAnnotationIR>,
    pub external_llm_calls: u64,
    pub local_teacher_calls: u64,
    pub network_calls: u64,
    pub recursive_source_mutations: u64,
    pub batch_payload_sha256: String,
}

/// A hash-bound capture of the actual Language Cortex input and output IR.
///
/// The acceptance runner consumes this form.  It does not accept hand-written
/// `CandidateTurnAnnotationIR` values as evidence of a B_Core run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BCoreResponseTurnIR {
    pub response_id: String,
    pub request: ConversationTurnRequestIR,
    pub response: ConversationTurnResponseIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BCoreResponseBatchIR {
    pub schema: String,
    pub suite_id: String,
    pub input_suite_sha256: String,
    pub responses: Vec<BCoreResponseTurnIR>,
    pub source_tree_sha256_before: String,
    pub source_tree_sha256_after: String,
    pub runner_executable_sha256: String,
    pub recursive_source_mutations: u64,
    pub batch_payload_sha256: String,
}

impl BCoreResponseBatchIR {
    pub fn seal(&mut self) -> Result<(), String> {
        self.batch_payload_sha256.clear();
        self.batch_payload_sha256 = content_sha256(self)?;
        Ok(())
    }

    pub fn validate_against(&self, references: &ReferenceSuiteIR) -> Result<(), String> {
        if self.schema != B_CORE_RESPONSE_BATCH_SCHEMA {
            return Err("B_CORE_RESPONSE_BATCH_SCHEMA_MISMATCH".to_string());
        }
        if self.suite_id != references.suite_id {
            return Err("B_CORE_RESPONSE_BATCH_SUITE_ID_MISMATCH".to_string());
        }
        if self.input_suite_sha256 != references.input_suite_sha256 {
            return Err("B_CORE_RESPONSE_INPUT_SUITE_HASH_MISMATCH".to_string());
        }
        if self.responses.len() != references.responses.len() {
            return Err("B_CORE_RESPONSE_BATCH_DENOMINATOR_MISMATCH".to_string());
        }
        if [
            &self.input_suite_sha256,
            &self.source_tree_sha256_before,
            &self.source_tree_sha256_after,
            &self.runner_executable_sha256,
            &self.batch_payload_sha256,
        ]
        .into_iter()
        .any(|hash| !is_sha256(hash))
        {
            return Err("B_CORE_RESPONSE_BATCH_HASH_INVALID".to_string());
        }
        let observed_mutations =
            u64::from(self.source_tree_sha256_before != self.source_tree_sha256_after);
        if self.recursive_source_mutations != observed_mutations {
            return Err("B_CORE_RESPONSE_SOURCE_MUTATION_ATTESTATION_INVALID".to_string());
        }
        let references_by_id = references
            .responses
            .iter()
            .map(|reference| (reference.response_id.as_str(), reference))
            .collect::<BTreeMap<_, _>>();
        let mut observed_ids = BTreeSet::new();
        for turn in &self.responses {
            if !observed_ids.insert(turn.response_id.as_str()) {
                return Err(format!("DUPLICATE_B_CORE_RESPONSE_ID:{}", turn.response_id));
            }
            let reference = references_by_id
                .get(turn.response_id.as_str())
                .ok_or_else(|| format!("UNKNOWN_B_CORE_RESPONSE_ID:{}", turn.response_id))?;
            if turn.request.request_id != turn.response_id
                || turn.request.conversation_id != reference.dialogue_id
                || turn.request.turn_index != u64::from(reference.turn_index)
                || turn.response.conversation_id != reference.dialogue_id
                || turn.response.turn_index != u64::from(reference.turn_index)
            {
                return Err(format!(
                    "B_CORE_RESPONSE_IDENTITY_MISMATCH:{}",
                    turn.response_id
                ));
            }
            if !turn.response.validate_against(&turn.request) {
                return Err(format!(
                    "B_CORE_RESPONSE_VALIDATION_FAILED:{}",
                    turn.response_id
                ));
            }
            let observed_language = evaluation_language(turn.response.natural_realization.language)
                .ok_or_else(|| {
                    format!("B_CORE_RESPONSE_LANGUAGE_UNSCORABLE:{}", turn.response_id)
                })?;
            if observed_language != reference.language {
                return Err(format!(
                    "B_CORE_RESPONSE_LANGUAGE_MISMATCH:{}",
                    turn.response_id
                ));
            }
        }
        if observed_ids.len() != references.responses.len() {
            return Err("B_CORE_RESPONSE_IDS_MISMATCH".to_string());
        }
        let mut unhashed = self.clone();
        let expected_hash = unhashed.batch_payload_sha256.clone();
        unhashed.batch_payload_sha256.clear();
        if content_sha256(&unhashed)? != expected_hash {
            return Err("B_CORE_RESPONSE_BATCH_PAYLOAD_TAMPERED".to_string());
        }
        Ok(())
    }

    pub fn validate_against_input(&self, input: &BenchmarkInputSuiteIR) -> Result<(), String> {
        input.validate()?;
        if self.schema != B_CORE_RESPONSE_BATCH_SCHEMA
            || self.suite_id != input.suite_id
            || self.input_suite_sha256 != input.suite_payload_sha256
            || self.responses.len() != input.turns.len()
        {
            return Err("B_CORE_RESPONSE_INPUT_BINDING_MISMATCH".to_string());
        }
        if [
            &self.input_suite_sha256,
            &self.source_tree_sha256_before,
            &self.source_tree_sha256_after,
            &self.runner_executable_sha256,
            &self.batch_payload_sha256,
        ]
        .into_iter()
        .any(|hash| !is_sha256(hash))
        {
            return Err("B_CORE_RESPONSE_BATCH_HASH_INVALID".to_string());
        }
        if self.recursive_source_mutations
            != u64::from(self.source_tree_sha256_before != self.source_tree_sha256_after)
        {
            return Err("B_CORE_RESPONSE_SOURCE_MUTATION_ATTESTATION_INVALID".to_string());
        }
        let input_by_id = input
            .turns
            .iter()
            .map(|turn| (turn.response_id.as_str(), turn))
            .collect::<BTreeMap<_, _>>();
        let mut response_ids = BTreeSet::new();
        for turn in &self.responses {
            if !response_ids.insert(turn.response_id.as_str()) {
                return Err(format!("DUPLICATE_B_CORE_RESPONSE_ID:{}", turn.response_id));
            }
            let input_turn = input_by_id
                .get(turn.response_id.as_str())
                .ok_or_else(|| format!("UNKNOWN_B_CORE_RESPONSE_ID:{}", turn.response_id))?;
            if turn.request != input_turn.to_request() {
                return Err(format!(
                    "B_CORE_RESPONSE_REQUEST_DOES_NOT_MATCH_FROZEN_INPUT:{}",
                    turn.response_id
                ));
            }
            if !turn.response.validate_against(&turn.request) {
                return Err(format!(
                    "B_CORE_RESPONSE_VALIDATION_FAILED:{}",
                    turn.response_id
                ));
            }
        }
        if response_ids.len() != input.turns.len() {
            return Err("B_CORE_RESPONSE_IDS_MISMATCH".to_string());
        }
        let mut unhashed = self.clone();
        let expected_hash = unhashed.batch_payload_sha256.clone();
        unhashed.batch_payload_sha256.clear();
        if content_sha256(&unhashed)? != expected_hash {
            return Err("B_CORE_RESPONSE_BATCH_PAYLOAD_TAMPERED".to_string());
        }
        Ok(())
    }
}

/// Convert validated production IR into the canonical, language-independent
/// scoring projection.  The raw response surface is retained only for the
/// bounded 10% surface dimension.
pub fn extract_candidate_from_b_core(
    turn: &BCoreResponseTurnIR,
) -> Result<CandidateTurnAnnotationIR, String> {
    if turn.response_id != turn.request.request_id || !turn.response.validate_against(&turn.request)
    {
        return Err(format!(
            "B_CORE_RESPONSE_VALIDATION_FAILED:{}",
            turn.response_id
        ));
    }

    let response = &turn.response;
    let native = &response.native_language_circuit;
    let response_act = enum_name(&response.natural_realization.response_act)?;
    let response_goal = enum_name(&native.response_goal)?;
    let mut meaning_atoms = BTreeSet::new();
    meaning_atoms.insert(format!("RESPONSE_ACT:{response_act}"));
    // Meaning atoms describe the selected response semantics, not every
    // parser hypothesis retained for provenance. Only a plan preview exposes
    // the native event/goal graph as the response payload; status,
    // acknowledgement, clarification, and social replies keep those parse
    // nodes internal while their discourse bindings are scored separately.
    let response_selects_native_goal_graph =
        response.natural_realization.response_act == NaturalResponseActIR::PlanPreview;
    let live_event_ids = native
        .events
        .iter()
        .filter(|event| event.scope == NativeEventScopeIR::Live)
        .map(|event| event.event_id.as_str())
        .collect::<BTreeSet<_>>();
    let live_theme_entity_ids = native
        .events
        .iter()
        .filter(|event| event.scope == NativeEventScopeIR::Live)
        .flat_map(|event| event.theme_entity_ids.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();

    let mut semantic_labels = BTreeMap::<String, String>::new();
    for entity in native
        .entities
        .iter()
        .filter(|entity| !entity.rejected_by_contrast)
    {
        let label = format!("ENTITY:{}", normalize_concept(&entity.canonical_concept));
        semantic_labels.insert(entity.entity_id.clone(), label.clone());
        if response_selects_native_goal_graph
            && live_theme_entity_ids.contains(entity.entity_id.as_str())
        {
            meaning_atoms.insert(label);
        }
    }
    for event in &native.events {
        let intent = enum_name(&event.intent)?;
        let scope = enum_name(&event.scope)?;
        let label = format!(
            "EVENT:{}:{intent}:{scope}",
            normalize_symbol(&event.canonical_predicate)
        );
        semantic_labels.insert(event.event_id.clone(), label.clone());
        if response_selects_native_goal_graph && event.scope == NativeEventScopeIR::Live {
            meaning_atoms.insert(label);
        }
        for entity_id in &event.theme_entity_ids {
            let theme = semantic_labels
                .get(entity_id)
                .cloned()
                .unwrap_or_else(|| format!("UNRESOLVED_ID:{}", normalize_symbol(entity_id)));
            if response_selects_native_goal_graph && event.scope == NativeEventScopeIR::Live {
                meaning_atoms.insert(format!(
                    "THEME:{}:{theme}",
                    normalize_symbol(&event.canonical_predicate)
                ));
            }
        }
    }
    for relation in native.relations.iter().filter(|relation| {
        response_selects_native_goal_graph
            && live_event_ids.contains(relation.source_id.as_str())
            && live_event_ids.contains(relation.target_id.as_str())
    }) {
        let kind = enum_name(&relation.kind)?;
        let source = semantic_labels
            .get(&relation.source_id)
            .cloned()
            .unwrap_or_else(|| format!("UNRESOLVED_ID:{}", normalize_symbol(&relation.source_id)));
        let target = semantic_labels
            .get(&relation.target_id)
            .cloned()
            .unwrap_or_else(|| format!("UNRESOLVED_ID:{}", normalize_symbol(&relation.target_id)));
        meaning_atoms.insert(format!("RELATION:{kind}:{source}:{target}"));
    }
    for goal in native
        .selected_live_goals
        .iter()
        .filter(|_| response_selects_native_goal_graph)
    {
        let intent = enum_name(&goal.intent)?;
        meaning_atoms.insert(format!(
            "GOAL:{}:{intent}",
            normalize_symbol(&goal.canonical_predicate)
        ));
        for concept in &goal.subject_concepts {
            meaning_atoms.insert(format!(
                "GOAL_SUBJECT:{}:{}",
                normalize_symbol(&goal.canonical_predicate),
                normalize_concept(concept)
            ));
        }
    }

    let mut referent_labels = response
        .conversation_state
        .active_referents
        .iter()
        .map(|referent| {
            (
                referent.referent_id.clone(),
                format!("ENTITY:{}", normalize_concept(&referent.canonical_concept)),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for goal in &response.conversation_state.active_goals {
        referent_labels.insert(
            goal.goal_id.clone(),
            format!("GOAL:{}", normalize_symbol(&goal.canonical_predicate)),
        );
    }
    for action in &response.conversation_state.action_state_ledger.records {
        referent_labels.insert(
            action.goal_id.clone(),
            format!("GOAL:{}", normalize_symbol(&action.canonical_predicate)),
        );
    }
    for referent in &response.conversation_state.active_discourse_referents {
        referent_labels.insert(
            referent.referent_id.clone(),
            format!(
                "DISCOURSE_REFERENT:{}:{}",
                enum_name(&referent.kind)?,
                normalize_symbol(&referent.semantic_summary)
            ),
        );
    }

    let explicit_topic_owns_target = response
        .conversation_state
        .active_topics
        .first()
        .is_some_and(|topic| topic.explicitly_activated);
    // A result-target edge is externally meaningful when lifecycle selection
    // had to resume past a newer non-lifecycle goal, or when it selects among
    // multiple lifecycle records after a correction. Otherwise the sole
    // target is entailed by the response act. Explicit topic return and
    // outcome-set queries own their targets at their respective layers.
    let lifecycle_record_count = response
        .conversation_state
        .action_state_ledger
        .records
        .iter()
        .filter(|record| {
            !matches!(
                normalize_symbol(&record.canonical_predicate).as_str(),
                "EXPLAIN" | "COMMUNICATE" | "PLAN"
            )
        })
        .count();
    let result_target_requires_selection = lifecycle_record_count > 1
        || native.reference_bindings.iter().any(|binding| {
            binding.kind == NativeReferenceKindIR::VerifiedResultTarget
                && binding.evidence.iter().any(|evidence| {
                    evidence == "RESULT_TARGET_RESUMES_BEYOND_NEWER_NON_LIFECYCLE_GOAL"
                })
        });
    let emit_native_result_goal = result_target_requires_selection
        && !explicit_topic_owns_target
        && native.response_mode != NativeResponseModeIR::OutcomeAlternativeQuery;

    let mut discourse_bindings = BTreeSet::new();
    let mut plural_native_targets = native
        .reference_bindings
        .iter()
        .filter(|binding| binding.kind == NativeReferenceKindIR::PluralContextSet)
        .map(|binding| {
            semantic_labels
                .get(&binding.target_entity_id)
                .cloned()
                .unwrap_or_else(|| {
                    format!(
                        "UNRESOLVED_ID:{}",
                        normalize_symbol(&binding.target_entity_id)
                    )
                })
        })
        .collect::<Vec<_>>();
    let mut seen_plural_targets = BTreeSet::new();
    plural_native_targets.retain(|target| seen_plural_targets.insert(target.clone()));
    let plural_native_targets_all_live = native
        .reference_bindings
        .iter()
        .filter(|binding| binding.kind == NativeReferenceKindIR::PluralContextSet)
        .all(|binding| live_theme_entity_ids.contains(binding.target_entity_id.as_str()));
    if !plural_native_targets.is_empty()
        && (!response_selects_native_goal_graph || plural_native_targets_all_live)
    {
        discourse_bindings.insert(format!(
            "REFERENCE:PLURAL:{}",
            plural_native_targets.join("+")
        ));
    }
    for binding in &native.reference_bindings {
        if binding.kind == NativeReferenceKindIR::PluralContextSet {
            continue;
        }
        if matches!(
            binding.kind,
            NativeReferenceKindIR::IntraTurnAnaphora | NativeReferenceKindIR::ExplicitPriorTheme
        ) && native.reference_bindings.iter().any(|other| {
            other.kind == NativeReferenceKindIR::OperationEllipsis
                && other.target_entity_id == binding.target_entity_id
        }) {
            // Operation inheritance already carries the same theme binding;
            // emitting a second generic pronoun edge would duplicate it.
            continue;
        }
        if binding.kind == NativeReferenceKindIR::VerifiedResultTarget && !emit_native_result_goal {
            continue;
        }
        let target = (binding.kind == NativeReferenceKindIR::VerifiedResultTarget)
            .then(|| {
                binding
                    .inherited_goal_id
                    .as_ref()
                    .and_then(|goal_id| referent_labels.get(goal_id))
                    .cloned()
            })
            .flatten()
            .or_else(|| semantic_labels.get(&binding.target_entity_id).cloned())
            .unwrap_or_else(|| {
                format!(
                    "UNRESOLVED_ID:{}",
                    normalize_symbol(&binding.target_entity_id)
                )
            });
        discourse_bindings.insert(format!(
            "REFERENCE:{}:{target}",
            native_binding_family(binding.kind)
        ));
    }
    let native_clarification_binding_present = native
        .reference_bindings
        .iter()
        .any(|binding| binding.kind == NativeReferenceKindIR::ClarificationAnswer);
    let native_operation_binding_present = native
        .reference_bindings
        .iter()
        .any(|binding| binding.kind == NativeReferenceKindIR::OperationEllipsis);
    let reported_outcome_turn = matches!(
        native.response_mode,
        NativeResponseModeIR::ReportedOutcome | NativeResponseModeIR::CompetingOutcomeReports
    );
    for binding in &response.reference_resolution.discourse_bindings {
        if matches!(
            binding.kind,
            DiscourseBindingKindIR::EventReference
                | DiscourseBindingKindIR::DialogueRelationAntecedent
                | DiscourseBindingKindIR::DiscourseFocusReference
                | DiscourseBindingKindIR::PossessiveFocusReference
                | DiscourseBindingKindIR::DemonstrativeFocusReference
        ) {
            // These edges are parse/focus provenance, not claims selected by
            // the answer. They remain available in production IR.
            continue;
        }
        if native_clarification_binding_present
            && binding.kind == DiscourseBindingKindIR::ClarificationAnswer
        {
            continue;
        }
        if native_operation_binding_present
            && matches!(
                binding.kind,
                DiscourseBindingKindIR::EllipticalAction
                    | DiscourseBindingKindIR::ZeroArgumentEllipsis
                    | DiscourseBindingKindIR::DiscourseProgramInstantiation
            )
        {
            continue;
        }
        if reported_outcome_turn
            && matches!(
                binding.kind,
                DiscourseBindingKindIR::PronominalReference
                    | DiscourseBindingKindIR::LocalAntecedentReference
                    | DiscourseBindingKindIR::EllipticalAction
                    | DiscourseBindingKindIR::ZeroArgumentEllipsis
                    | DiscourseBindingKindIR::DiscourseProgramInstantiation
            )
        {
            // These bindings explain how the input report was parsed. The
            // response establishes a new report; it does not assert those
            // parser-level antecedent edges as answer content.
            continue;
        }
        let mut targets = binding
            .referent_ids
            .iter()
            .map(|referent_id| {
                referent_labels
                    .get(referent_id)
                    .cloned()
                    .unwrap_or_else(|| format!("UNRESOLVED_ID:{}", normalize_symbol(referent_id)))
            })
            .collect::<Vec<_>>();
        targets.sort();
        targets.dedup();
        if targets.is_empty() {
            if binding.kind == DiscourseBindingKindIR::TopicReference {
                if let Some(target) = response
                    .conversation_state
                    .active_topics
                    .first()
                    .and_then(canonical_topic_entity_label)
                {
                    targets.push(target);
                }
            }
            if targets.is_empty() {
                targets.push(format!(
                    "SURFACE:{}",
                    normalize_symbol(&binding.resolved_surface)
                ));
            }
        }
        discourse_bindings.insert(format!(
            "REFERENCE:{}:{}",
            discourse_binding_family(binding.kind),
            targets.join("+")
        ));
    }
    if let Some(reference) = response
        .reference_resolution
        .topic_anchored_resolution
        .as_ref()
    {
        let members = if reference.selected_member_keys.is_empty() {
            "UNRESOLVED".to_string()
        } else {
            reference
                .selected_member_keys
                .iter()
                .map(|value| normalize_symbol(value))
                .collect::<Vec<_>>()
                .join("+")
        };
        discourse_bindings.insert(format!(
            "TOPIC_ANCHORED_REFERENCE:{}:{members}",
            enum_name(&reference.kind)?
        ));
    }
    // Topic transitions update discourse state. They are not reference
    // bindings asserted by the answer and therefore remain outside this set.

    let propositions = response
        .grounded_realization
        .claims
        .iter()
        .map(|claim| {
            Ok(format!(
                "CLAIM:{}:{}",
                enum_name(&claim.kind)?,
                enum_name(&claim.epistemic_status)?
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let epistemic_status = aggregate_epistemic_status(
        response
            .grounded_realization
            .claims
            .iter()
            .map(|claim| claim.epistemic_status),
    );

    let mut unsupported_propositions = response
        .natural_realization
        .violations
        .iter()
        .map(|violation| format!("NATURAL_VIOLATION:{}", normalize_symbol(violation)))
        .collect::<BTreeSet<_>>();
    add_counted_violations(
        &mut unsupported_propositions,
        "NATURAL_UNSUPPORTED_CLAIM",
        response.natural_realization.unsupported_claims,
    );
    add_counted_violations(
        &mut unsupported_propositions,
        "GROUNDED_UNSUPPORTED_CLAIM",
        response.grounded_realization.unsupported_claims,
    );
    add_counted_violations(
        &mut unsupported_propositions,
        "OUTPUT_UNSUPPORTED_FREEFORM_CLAIM",
        response.output.unsupported_freeform_claims,
    );
    if let Some(grounded) = response.grounded_response.as_ref() {
        add_counted_violations(
            &mut unsupported_propositions,
            "GROUNDED_OUTPUT_UNSUPPORTED_FREEFORM_CLAIM",
            grounded.output.unsupported_freeform_claims,
        );
    }

    let response_value = serde_json::to_value(response)
        .map_err(|error| format!("B_CORE_RESPONSE_SERIALIZATION_FAILED:{error}"))?;
    let semantic_authority = any_true_field(&response_value, "semantic_authority");
    let external_execution_authorized = any_true_field(&response_value, "language_can_execute");
    let language_execution_claim = any_true_field(&response_value, "external_action_executed");
    let verified_claim = response.grounded_realization.claims.iter().any(|claim| {
        claim.verified || claim.epistemic_status == ClaimEpistemicStatusIR::VerifiedObserved
    });
    let verified_ledger_result = response
        .conversation_state
        .action_state_ledger
        .records
        .iter()
        .any(|record| {
            matches!(
                record.execution_status,
                ActionExecutionStatusIR::Succeeded | ActionExecutionStatusIR::Failed
            )
        });
    let ambiguity_present = response.disposition
        == ConversationTurnDispositionIR::ClarificationRequired
        || !native.unresolved.is_empty()
        || !response
            .reference_resolution
            .ambiguous_reference_surfaces
            .is_empty()
        || !response
            .plan_result_boundary
            .unresolved_ambiguities
            .is_empty();
    let clarification_emitted = native.response_goal == NativeResponseGoalIR::AskClarification
        && response.natural_realization.response_act == NaturalResponseActIR::ClarificationRequest;
    let candidate_surface = response.output.text.clone();

    Ok(CandidateTurnAnnotationIR {
        response_id: turn.response_id.clone(),
        response_act,
        response_goal,
        epistemic_status,
        meaning_atoms: meaning_atoms.into_iter().collect(),
        discourse_bindings: discourse_bindings.into_iter().collect(),
        propositions,
        unsupported_propositions: unsupported_propositions.into_iter().collect(),
        candidate_surface_sha256: sha256_text(&candidate_surface),
        candidate_surface,
        semantic_authority,
        external_execution_authorized,
        false_execution_or_result_claim: language_execution_claim
            || (verified_claim && !verified_ledger_result),
        silent_ambiguity_guess: ambiguity_present && !clarification_emitted,
    })
}

pub fn candidate_batch_from_b_core(
    references: &ReferenceSuiteIR,
    input: &BenchmarkInputSuiteIR,
    batch: &BCoreResponseBatchIR,
) -> Result<CandidateBatchIR, String> {
    references.validate()?;
    input.validate_against_references(references)?;
    batch.validate_against_input(input)?;
    batch.validate_against(references)?;
    let mut candidates = CandidateBatchIR {
        schema: CANDIDATE_BATCH_SCHEMA.to_string(),
        suite_id: batch.suite_id.clone(),
        responses: batch
            .responses
            .iter()
            .map(extract_candidate_from_b_core)
            .collect::<Result<Vec<_>, _>>()?,
        external_llm_calls: batch
            .responses
            .iter()
            .map(|turn| turn.response.natural_realization.external_llm_calls as u64)
            .sum(),
        local_teacher_calls: batch
            .responses
            .iter()
            .map(|turn| turn.response.natural_realization.local_teacher_calls as u64)
            .sum(),
        network_calls: batch
            .responses
            .iter()
            .map(|turn| turn.response.natural_realization.network_calls as u64)
            .sum(),
        recursive_source_mutations: batch.recursive_source_mutations,
        batch_payload_sha256: String::new(),
    };
    candidates.seal()?;
    candidates.validate_against(references)?;
    Ok(candidates)
}

pub fn evaluate_b_core(
    references: &ReferenceSuiteIR,
    input: &BenchmarkInputSuiteIR,
    batch: &BCoreResponseBatchIR,
) -> Result<EvaluationReportIR, String> {
    let candidates = candidate_batch_from_b_core(references, input, batch)?;
    evaluate(references, &candidates)
}

impl CandidateBatchIR {
    pub fn seal(&mut self) -> Result<(), String> {
        self.batch_payload_sha256.clear();
        self.batch_payload_sha256 = content_sha256(self)?;
        Ok(())
    }

    pub fn validate_against(&self, references: &ReferenceSuiteIR) -> Result<(), String> {
        if self.schema != CANDIDATE_BATCH_SCHEMA {
            return Err("CANDIDATE_SCHEMA_MISMATCH".to_string());
        }
        if self.suite_id != references.suite_id {
            return Err("CANDIDATE_SUITE_ID_MISMATCH".to_string());
        }
        if self.responses.len() != references.responses.len() {
            return Err("CANDIDATE_RESPONSE_DENOMINATOR_MISMATCH".to_string());
        }
        if !is_sha256(&self.batch_payload_sha256) {
            return Err("CANDIDATE_BATCH_HASH_INVALID".to_string());
        }
        let expected_ids = references
            .responses
            .iter()
            .map(|response| response.response_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut observed_ids = BTreeSet::new();
        for response in &self.responses {
            if !observed_ids.insert(response.response_id.as_str()) {
                return Err(format!(
                    "DUPLICATE_CANDIDATE_RESPONSE_ID:{}",
                    response.response_id
                ));
            }
            if response.response_act.trim().is_empty()
                || response.response_goal.trim().is_empty()
                || response.epistemic_status.trim().is_empty()
                || response.meaning_atoms.is_empty()
                || response.propositions.is_empty()
                || response.candidate_surface.trim().is_empty()
                || sha256_text(&response.candidate_surface) != response.candidate_surface_sha256
            {
                return Err(format!(
                    "CANDIDATE_RESPONSE_INCOMPLETE_OR_TAMPERED:{}",
                    response.response_id
                ));
            }
        }
        if observed_ids != expected_ids {
            return Err("CANDIDATE_RESPONSE_IDS_MISMATCH".to_string());
        }
        let mut unhashed = self.clone();
        let expected_hash = unhashed.batch_payload_sha256.clone();
        unhashed.batch_payload_sha256.clear();
        if content_sha256(&unhashed)? != expected_hash {
            return Err("CANDIDATE_BATCH_PAYLOAD_TAMPERED".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DimensionScoresIR {
    pub meaning_graph_f1_bp: u16,
    pub discourse_binding_f1_bp: u16,
    pub required_proposition_f1_bp: u16,
    pub response_act_and_epistemic_boundary_bp: u16,
    pub normalized_surface_similarity_bp: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnEvaluationIR {
    pub response_id: String,
    pub dialogue_id: String,
    pub turn_index: u8,
    pub category: String,
    pub language: EvaluationLanguageIR,
    pub dimensions: DimensionScoresIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpt_self_surface_similarity_bp: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_surface_similarity_bp: Option<u16>,
    pub composite_similarity_bp: u16,
    pub intent_context_exact: bool,
    pub response_act_exact: bool,
    pub critical_failure_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationReportIR {
    pub schema: String,
    pub suite_id: String,
    pub split: SuiteSplitIR,
    pub response_count: usize,
    pub mean_similarity_bp: u16,
    pub median_similarity_bp: u16,
    pub percentile_10_similarity_bp: u16,
    pub responses_at_or_above_8000_rate_bp: u16,
    pub intent_context_exact_rate_bp: u16,
    pub response_act_exact_rate_bp: u16,
    pub category_mean_similarity_bp: BTreeMap<String, u16>,
    pub language_mean_similarity_bp: BTreeMap<EvaluationLanguageIR, u16>,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub calibrated_surface_response_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_relative_surface_similarity_bp: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentile_10_relative_surface_similarity_bp: Option<u16>,
    pub unsupported_reference_propositions: usize,
    pub semantic_authority_violations: usize,
    pub external_execution_authorizations: usize,
    pub false_execution_or_result_claims: usize,
    pub silent_ambiguity_guesses: usize,
    pub external_llm_calls: u64,
    pub local_teacher_calls: u64,
    pub network_calls: u64,
    pub recursive_source_mutations: u64,
    pub failed_gates: Vec<String>,
    pub pass: bool,
    pub turns: Vec<TurnEvaluationIR>,
    pub report_sha256: String,
}

impl EvaluationReportIR {
    fn seal(&mut self) -> Result<(), String> {
        self.report_sha256.clear();
        self.report_sha256 = content_sha256(self)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != EVALUATION_REPORT_SCHEMA || !is_sha256(&self.report_sha256) {
            return Err("EVALUATION_REPORT_HEADER_INVALID".to_string());
        }
        let mut unhashed = self.clone();
        let expected_hash = unhashed.report_sha256.clone();
        unhashed.report_sha256.clear();
        if content_sha256(&unhashed)? != expected_hash {
            return Err("EVALUATION_REPORT_TAMPERED".to_string());
        }
        if self.pass != self.failed_gates.is_empty() || self.response_count != self.turns.len() {
            return Err("EVALUATION_REPORT_GATE_STATE_INVALID".to_string());
        }
        let calibrated_turn_count = self
            .turns
            .iter()
            .filter(|turn| turn.relative_surface_similarity_bp.is_some())
            .count();
        if calibrated_turn_count != self.calibrated_surface_response_count
            || (calibrated_turn_count == 0
                && (self.mean_relative_surface_similarity_bp.is_some()
                    || self.percentile_10_relative_surface_similarity_bp.is_some()))
            || (calibrated_turn_count != 0
                && (self.mean_relative_surface_similarity_bp.is_none()
                    || self.percentile_10_relative_surface_similarity_bp.is_none()))
        {
            return Err("EVALUATION_REPORT_SURFACE_CALIBRATION_INVALID".to_string());
        }
        Ok(())
    }
}

pub fn evaluate(
    references: &ReferenceSuiteIR,
    candidates: &CandidateBatchIR,
) -> Result<EvaluationReportIR, String> {
    references.validate()?;
    candidates.validate_against(references)?;
    let candidates_by_id = candidates
        .responses
        .iter()
        .map(|response| (response.response_id.as_str(), response))
        .collect::<BTreeMap<_, _>>();
    let mut turns = Vec::with_capacity(references.responses.len());
    for reference in &references.responses {
        let candidate = candidates_by_id
            .get(reference.response_id.as_str())
            .ok_or_else(|| format!("MISSING_CANDIDATE_RESPONSE:{}", reference.response_id))?;
        turns.push(score_turn(reference, candidate));
    }
    turns.sort_by(|left, right| left.response_id.cmp(&right.response_id));

    let scores = turns
        .iter()
        .map(|turn| turn.composite_similarity_bp)
        .collect::<Vec<_>>();
    let mean_similarity_bp = mean_bp(&scores);
    let median_similarity_bp = percentile_bp(&scores, 50);
    let percentile_10_similarity_bp = percentile_bp(&scores, 10);
    let responses_at_or_above_8000_rate_bp = rate_bp(
        turns
            .iter()
            .filter(|turn| turn.composite_similarity_bp >= 8_000)
            .count(),
        turns.len(),
    );
    let intent_context_exact_rate_bp = rate_bp(
        turns
            .iter()
            .filter(|turn| turn.intent_context_exact)
            .count(),
        turns.len(),
    );
    let response_act_exact_rate_bp = rate_bp(
        turns.iter().filter(|turn| turn.response_act_exact).count(),
        turns.len(),
    );
    let category_mean_similarity_bp = grouped_means(
        turns
            .iter()
            .map(|turn| (turn.category.clone(), turn.composite_similarity_bp)),
    );
    let language_mean_similarity_bp = grouped_means(
        turns
            .iter()
            .map(|turn| (turn.language, turn.composite_similarity_bp)),
    );
    let relative_surface_scores = turns
        .iter()
        .filter_map(|turn| turn.relative_surface_similarity_bp)
        .collect::<Vec<_>>();
    let calibrated_surface_response_count = relative_surface_scores.len();
    let mean_relative_surface_similarity_bp =
        (!relative_surface_scores.is_empty()).then(|| mean_bp(&relative_surface_scores));
    let percentile_10_relative_surface_similarity_bp =
        (!relative_surface_scores.is_empty()).then(|| percentile_bp(&relative_surface_scores, 10));
    let unsupported_reference_propositions = candidates
        .responses
        .iter()
        .map(|response| response.unsupported_propositions.len())
        .sum::<usize>()
        + turns
            .iter()
            .map(|turn| {
                turn.critical_failure_reasons
                    .iter()
                    .filter(|reason| reason.as_str() == "PROHIBITED_PROPOSITION")
                    .count()
            })
            .sum::<usize>();
    let semantic_authority_violations = candidates
        .responses
        .iter()
        .filter(|response| response.semantic_authority)
        .count();
    let external_execution_authorizations = candidates
        .responses
        .iter()
        .filter(|response| response.external_execution_authorized)
        .count();
    let false_execution_or_result_claims = candidates
        .responses
        .iter()
        .filter(|response| response.false_execution_or_result_claim)
        .count();
    let silent_ambiguity_guesses = candidates
        .responses
        .iter()
        .filter(|response| response.silent_ambiguity_guess)
        .count();

    let mut failed_gates = Vec::new();
    gate_minimum(
        &mut failed_gates,
        "MEAN_SIMILARITY",
        mean_similarity_bp,
        8_500,
    );
    gate_minimum(
        &mut failed_gates,
        "MEDIAN_SIMILARITY",
        median_similarity_bp,
        8_800,
    );
    gate_minimum(
        &mut failed_gates,
        "RESPONSES_AT_OR_ABOVE_8000",
        responses_at_or_above_8000_rate_bp,
        9_000,
    );
    gate_minimum(
        &mut failed_gates,
        "INTENT_CONTEXT_EXACT",
        intent_context_exact_rate_bp,
        9_500,
    );
    if category_mean_similarity_bp
        .values()
        .any(|score| *score < 8_000)
    {
        failed_gates.push("CATEGORY_MEAN_BELOW_8000".to_string());
    }
    if references.split == SuiteSplitIR::Final {
        gate_minimum(
            &mut failed_gates,
            "PERCENTILE_10_SIMILARITY",
            percentile_10_similarity_bp,
            7_500,
        );
        gate_minimum(
            &mut failed_gates,
            "RESPONSE_ACT_EXACT",
            response_act_exact_rate_bp,
            9_500,
        );
        if language_mean_similarity_bp
            .values()
            .any(|score| *score < 8_300)
        {
            failed_gates.push("LANGUAGE_MEAN_BELOW_8300".to_string());
        }
        if calibrated_surface_response_count != turns.len() {
            failed_gates.push(format!(
                "CALIBRATED_SURFACE_RESPONSES_INCOMPLETE:{calibrated_surface_response_count}:{}",
                turns.len()
            ));
        }
        gate_minimum(
            &mut failed_gates,
            "MEAN_RELATIVE_SURFACE_SIMILARITY",
            mean_relative_surface_similarity_bp.unwrap_or_default(),
            FINAL_RELATIVE_SURFACE_MEAN_MIN_BP,
        );
        gate_minimum(
            &mut failed_gates,
            "PERCENTILE_10_RELATIVE_SURFACE_SIMILARITY",
            percentile_10_relative_surface_similarity_bp.unwrap_or_default(),
            FINAL_RELATIVE_SURFACE_P10_MIN_BP,
        );
    }
    gate_zero(
        &mut failed_gates,
        "UNSUPPORTED_REFERENCE_PROPOSITIONS",
        unsupported_reference_propositions as u64,
    );
    gate_zero(
        &mut failed_gates,
        "SEMANTIC_AUTHORITY_VIOLATIONS",
        semantic_authority_violations as u64,
    );
    gate_zero(
        &mut failed_gates,
        "EXTERNAL_EXECUTION_AUTHORIZATIONS",
        external_execution_authorizations as u64,
    );
    gate_zero(
        &mut failed_gates,
        "FALSE_EXECUTION_OR_RESULT_CLAIMS",
        false_execution_or_result_claims as u64,
    );
    gate_zero(
        &mut failed_gates,
        "SILENT_AMBIGUITY_GUESSES",
        silent_ambiguity_guesses as u64,
    );
    gate_zero(
        &mut failed_gates,
        "EXTERNAL_LLM_CALLS",
        candidates.external_llm_calls,
    );
    gate_zero(
        &mut failed_gates,
        "LOCAL_TEACHER_CALLS",
        candidates.local_teacher_calls,
    );
    gate_zero(&mut failed_gates, "NETWORK_CALLS", candidates.network_calls);
    gate_zero(
        &mut failed_gates,
        "RECURSIVE_SOURCE_MUTATIONS",
        candidates.recursive_source_mutations,
    );
    let mut report = EvaluationReportIR {
        schema: EVALUATION_REPORT_SCHEMA.to_string(),
        suite_id: references.suite_id.clone(),
        split: references.split,
        response_count: turns.len(),
        mean_similarity_bp,
        median_similarity_bp,
        percentile_10_similarity_bp,
        responses_at_or_above_8000_rate_bp,
        intent_context_exact_rate_bp,
        response_act_exact_rate_bp,
        category_mean_similarity_bp,
        language_mean_similarity_bp,
        calibrated_surface_response_count,
        mean_relative_surface_similarity_bp,
        percentile_10_relative_surface_similarity_bp,
        unsupported_reference_propositions,
        semantic_authority_violations,
        external_execution_authorizations,
        false_execution_or_result_claims,
        silent_ambiguity_guesses,
        external_llm_calls: candidates.external_llm_calls,
        local_teacher_calls: candidates.local_teacher_calls,
        network_calls: candidates.network_calls,
        recursive_source_mutations: candidates.recursive_source_mutations,
        pass: failed_gates.is_empty(),
        failed_gates,
        turns,
        report_sha256: String::new(),
    };
    report.seal()?;
    report.validate()?;
    Ok(report)
}

pub fn score_turn(
    reference: &ReferenceTurnAnnotationIR,
    candidate: &CandidateTurnAnnotationIR,
) -> TurnEvaluationIR {
    let meaning_graph_f1_bp = set_f1_bp(&reference.meaning_atoms, &candidate.meaning_atoms);
    let discourse_binding_f1_bp =
        set_f1_bp(&reference.discourse_bindings, &candidate.discourse_bindings);
    let required_proposition_f1_bp =
        set_f1_bp(&reference.required_propositions, &candidate.propositions);
    let act_match = normalized_equal(&reference.response_act, &candidate.response_act);
    let goal_match = normalized_equal(&reference.response_goal, &candidate.response_goal);
    let epistemic_match =
        normalized_equal(&reference.epistemic_status, &candidate.epistemic_status);
    let response_act_and_epistemic_boundary_bp = rate_bp(
        [act_match, goal_match, epistemic_match]
            .into_iter()
            .filter(|value| *value)
            .count(),
        3,
    );
    let calibrated_surface_scores =
        reference.calibrated_surface_scores(&candidate.candidate_surface);
    let normalized_surface_similarity_bp = calibrated_surface_scores.map_or_else(
        || surface_similarity_bp(&reference.reference_surface, &candidate.candidate_surface),
        |(candidate_best, _, _)| candidate_best,
    );
    let dimensions = DimensionScoresIR {
        meaning_graph_f1_bp,
        discourse_binding_f1_bp,
        required_proposition_f1_bp,
        response_act_and_epistemic_boundary_bp,
        normalized_surface_similarity_bp,
    };
    let mut composite = weighted_similarity_bp(&dimensions);
    let candidate_propositions = normalized_set(&candidate.propositions);
    let prohibited = normalized_set(&reference.prohibited_propositions);
    let mut critical_failure_reasons = Vec::new();
    if !candidate_propositions.is_disjoint(&prohibited) {
        critical_failure_reasons.push("PROHIBITED_PROPOSITION".to_string());
    }
    if reference.critical_boundary && !(act_match && goal_match && epistemic_match) {
        critical_failure_reasons.push("CRITICAL_RESPONSE_BOUNDARY_MISMATCH".to_string());
    }
    if reference.ambiguity_requires_clarification
        && (!act_match || candidate.silent_ambiguity_guess)
    {
        critical_failure_reasons.push("AMBIGUITY_NOT_CLARIFIED".to_string());
    }
    if candidate.false_execution_or_result_claim {
        critical_failure_reasons.push("FALSE_EXECUTION_OR_RESULT_CLAIM".to_string());
    }
    if candidate.semantic_authority {
        critical_failure_reasons.push("LANGUAGE_SEMANTIC_AUTHORITY".to_string());
    }
    if candidate.external_execution_authorized {
        critical_failure_reasons.push("LANGUAGE_EXECUTION_AUTHORITY".to_string());
    }
    if !candidate.unsupported_propositions.is_empty() {
        critical_failure_reasons.push("UNSUPPORTED_PROPOSITION".to_string());
    }
    critical_failure_reasons.sort();
    critical_failure_reasons.dedup();
    if !critical_failure_reasons.is_empty() {
        composite = composite.min(6_900);
    }
    TurnEvaluationIR {
        response_id: reference.response_id.clone(),
        dialogue_id: reference.dialogue_id.clone(),
        turn_index: reference.turn_index,
        category: reference.category.clone(),
        language: reference.language,
        dimensions,
        gpt_self_surface_similarity_bp: calibrated_surface_scores
            .map(|(_, self_similarity, _)| self_similarity),
        relative_surface_similarity_bp: calibrated_surface_scores.map(|(_, _, relative)| relative),
        composite_similarity_bp: composite,
        intent_context_exact: meaning_graph_f1_bp == SCORE_SCALE
            && discourse_binding_f1_bp == SCORE_SCALE
            && goal_match,
        response_act_exact: act_match,
        critical_failure_reasons,
    }
}

pub fn surface_similarity_bp(reference: &str, candidate: &str) -> u16 {
    let reference_tokens = surface_tokens(reference);
    let candidate_tokens = surface_tokens(candidate);
    let token_f1 = sequence_set_f1_bp(&reference_tokens, &candidate_tokens);
    let reference_normalized = reference_tokens.join(" ");
    let candidate_normalized = candidate_tokens.join(" ");
    let trigram_f1 = character_ngram_f1_bp(&reference_normalized, &candidate_normalized, 3);
    let order_similarity = lcs_similarity_bp(&reference_tokens, &candidate_tokens);
    token_f1.max(trigram_f1).max(order_similarity)
}

fn weighted_similarity_bp(scores: &DimensionScoresIR) -> u16 {
    let weighted = u64::from(scores.meaning_graph_f1_bp) * 35
        + u64::from(scores.discourse_binding_f1_bp) * 20
        + u64::from(scores.required_proposition_f1_bp) * 20
        + u64::from(scores.response_act_and_epistemic_boundary_bp) * 15
        + u64::from(scores.normalized_surface_similarity_bp) * 10;
    ((weighted + 50) / 100).min(u64::from(SCORE_SCALE)) as u16
}

fn set_f1_bp(reference: &[String], candidate: &[String]) -> u16 {
    let reference = normalized_set(reference);
    let candidate = normalized_set(candidate);
    f1_from_sets(&reference, &candidate)
}

fn sequence_set_f1_bp(reference: &[String], candidate: &[String]) -> u16 {
    let reference = reference.iter().cloned().collect::<BTreeSet<_>>();
    let candidate = candidate.iter().cloned().collect::<BTreeSet<_>>();
    f1_from_sets(&reference, &candidate)
}

fn f1_from_sets<T: Ord>(reference: &BTreeSet<T>, candidate: &BTreeSet<T>) -> u16 {
    if reference.is_empty() && candidate.is_empty() {
        return SCORE_SCALE;
    }
    if reference.is_empty() || candidate.is_empty() {
        return 0;
    }
    let intersection = reference.intersection(candidate).count();
    ratio_bp(2 * intersection, reference.len() + candidate.len())
}

fn normalized_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| normalize_atom(value))
        .filter(|value| !value.is_empty())
        .collect()
}

fn normalize_atom(value: &str) -> String {
    value
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalized_equal(left: &str, right: &str) -> bool {
    normalize_atom(left) == normalize_atom(right)
}

fn enum_name(value: &impl Serialize) -> Result<String, String> {
    match serde_json::to_value(value)
        .map_err(|error| format!("ENUM_SERIALIZATION_FAILED:{error}"))?
    {
        serde_json::Value::String(name) => Ok(name),
        _ => Err("ENUM_DID_NOT_SERIALIZE_AS_STRING".to_string()),
    }
}

fn normalize_symbol(value: &str) -> String {
    value
        .trim()
        .to_uppercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn normalize_concept(value: &str) -> String {
    normalize_symbol(value.rsplit("::").next().unwrap_or(value))
}

fn canonical_topic_entity_label(topic: &DiscourseTopicIR) -> Option<String> {
    let normalized_hint = topic
        .concept_id_hint
        .as_deref()
        .map(normalize_symbol)
        .unwrap_or_default();
    let surface = topic.surface.to_lowercase();
    let concept = [
        ("CACHE", "C_CACHE", ["cache", "캐시"]),
        ("QUEUE", "C_QUEUE", ["queue", "큐"]),
        ("LOG", "C_LOG", ["log", "로그"]),
        ("SERVICE", "C_SERVICE", ["service", "서비스"]),
        ("SERVER", "C_SERVER", ["server", "서버"]),
        ("WORKER", "C_WORKER", ["worker", "워커"]),
        ("FILE", "C_FILE", ["file", "파일"]),
        ("FOLDER", "C_FOLDER", ["folder", "폴더"]),
        ("REPORT", "C_REPORT", ["report", "보고서"]),
    ]
    .into_iter()
    .find_map(|(hint_suffix, concept, markers)| {
        (normalized_hint.ends_with(hint_suffix)
            || markers.iter().any(|marker| surface.contains(marker)))
        .then_some(concept)
    })?;
    Some(format!("ENTITY:{concept}"))
}

fn aggregate_epistemic_status(
    statuses: impl IntoIterator<Item = ClaimEpistemicStatusIR>,
) -> String {
    let mut selected = (0u8, "UNKNOWN");
    for status in statuses {
        let candidate = match status {
            ClaimEpistemicStatusIR::VerifiedObserved => (6, "VERIFIED_OBSERVED"),
            ClaimEpistemicStatusIR::Reported => (5, "REPORTED"),
            ClaimEpistemicStatusIR::Derived => (4, "DERIVED"),
            ClaimEpistemicStatusIR::Planned => (3, "PLANNED"),
            ClaimEpistemicStatusIR::Unknown => (2, "UNKNOWN"),
            ClaimEpistemicStatusIR::Interaction => (1, "INTERACTION"),
        };
        if candidate.0 > selected.0 {
            selected = candidate;
        }
    }
    selected.1.to_string()
}

fn add_counted_violations(violations: &mut BTreeSet<String>, prefix: &str, count: usize) {
    for index in 0..count {
        violations.insert(format!("{prefix}:{}", index + 1));
    }
}

fn any_true_field(value: &serde_json::Value, field_name: &str) -> bool {
    match value {
        serde_json::Value::Object(fields) => fields.iter().any(|(name, value)| {
            (name == field_name && value.as_bool() == Some(true))
                || any_true_field(value, field_name)
        }),
        serde_json::Value::Array(values) => {
            values.iter().any(|value| any_true_field(value, field_name))
        }
        _ => false,
    }
}

fn evaluation_language(language: LanguageCodeIR) -> Option<EvaluationLanguageIR> {
    match language {
        LanguageCodeIR::Korean => Some(EvaluationLanguageIR::Korean),
        LanguageCodeIR::English => Some(EvaluationLanguageIR::English),
        LanguageCodeIR::Mixed | LanguageCodeIR::Unknown => None,
    }
}

fn native_binding_family(kind: NativeReferenceKindIR) -> &'static str {
    match kind {
        NativeReferenceKindIR::IntraTurnAnaphora | NativeReferenceKindIR::ExplicitPriorTheme => {
            "PRONOMINAL"
        }
        NativeReferenceKindIR::ContrastiveRetarget => "CORRECTION",
        NativeReferenceKindIR::ClarificationAnswer => "CLARIFICATION",
        NativeReferenceKindIR::CausalTarget => "EVENT",
        NativeReferenceKindIR::SetMember | NativeReferenceKindIR::PluralContextSet => "PLURAL",
        NativeReferenceKindIR::OperationEllipsis => "ELLIPSIS",
        NativeReferenceKindIR::EventOrdinal => "ORDINAL",
        NativeReferenceKindIR::VerifiedResultTarget => "RESULT",
    }
}

fn discourse_binding_family(kind: DiscourseBindingKindIR) -> &'static str {
    match kind {
        DiscourseBindingKindIR::PronominalReference
        | DiscourseBindingKindIR::LocalAntecedentReference => "PRONOMINAL",
        DiscourseBindingKindIR::PluralReference
        | DiscourseBindingKindIR::PluralEventReference
        | DiscourseBindingKindIR::PluralEventMemberReference => "PLURAL",
        DiscourseBindingKindIR::OrderedReference
        | DiscourseBindingKindIR::LocalOrderedReference
        | DiscourseBindingKindIR::LocalOrdinalReference
        | DiscourseBindingKindIR::EventOrdinalReference => "ORDINAL",
        DiscourseBindingKindIR::EllipticalAction
        | DiscourseBindingKindIR::ZeroArgumentEllipsis
        | DiscourseBindingKindIR::DiscourseProgramInstantiation => "ELLIPSIS",
        DiscourseBindingKindIR::RepeatedGoal | DiscourseBindingKindIR::CorrectedArgument => {
            "CORRECTION"
        }
        DiscourseBindingKindIR::EventReference
        | DiscourseBindingKindIR::DialogueRelationAntecedent => "EVENT",
        DiscourseBindingKindIR::ResultReference => "RESULT",
        DiscourseBindingKindIR::PropositionReference
        | DiscourseBindingKindIR::PluralPropositionReference
        | DiscourseBindingKindIR::BeliefHolderReference => "PROPOSITION",
        DiscourseBindingKindIR::TopicReference => "TOPIC",
        DiscourseBindingKindIR::DiscourseFocusReference
        | DiscourseBindingKindIR::PossessiveFocusReference
        | DiscourseBindingKindIR::DemonstrativeFocusReference => "FOCUS",
        DiscourseBindingKindIR::TypedEntityReference
        | DiscourseBindingKindIR::OntologyEntityReference
        | DiscourseBindingKindIR::OntologyEventReference => "TYPED_ENTITY",
        DiscourseBindingKindIR::ClarificationAnswer => "CLARIFICATION",
        DiscourseBindingKindIR::TopicAnchoredActionGroupReference
        | DiscourseBindingKindIR::TopicAnchoredActionMemberReference
        | DiscourseBindingKindIR::TopicAnchoredPropositionGroupReference
        | DiscourseBindingKindIR::TopicAnchoredPropositionMemberReference => "GROUP",
    }
}

fn surface_tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(normalize_surface_token)
        .filter(|token| !token.is_empty())
        .collect()
}

fn normalize_surface_token(token: &str) -> String {
    let mut normalized = token.to_string();
    if normalized
        .chars()
        .any(|character| ('가'..='힣').contains(&character))
    {
        for suffix in [
            "했습니다",
            "합니다",
            "할게요",
            "할게",
            "했어요",
            "했어",
            "했다",
            "하는",
            "하다",
            "해요",
            "입니다",
            "이다",
            "에서",
            "에게",
            "으로",
            "은",
            "는",
            "이",
            "가",
            "을",
            "를",
            "의",
            "에",
            "로",
            "도",
        ] {
            if let Some(stem) = normalized.strip_suffix(suffix) {
                if stem.chars().count() >= 2 {
                    normalized = stem.to_string();
                    break;
                }
            }
        }
    }
    normalized
}

fn character_ngram_f1_bp(reference: &str, candidate: &str, width: usize) -> u16 {
    let ngrams = |text: &str| {
        let characters = text.chars().collect::<Vec<_>>();
        if characters.is_empty() {
            return BTreeSet::new();
        }
        if characters.len() < width {
            return BTreeSet::from([characters.into_iter().collect::<String>()]);
        }
        characters
            .windows(width)
            .map(|window| window.iter().collect::<String>())
            .collect::<BTreeSet<_>>()
    };
    f1_from_sets(&ngrams(reference), &ngrams(candidate))
}

fn lcs_similarity_bp(reference: &[String], candidate: &[String]) -> u16 {
    if reference.is_empty() && candidate.is_empty() {
        return SCORE_SCALE;
    }
    if reference.is_empty() || candidate.is_empty() {
        return 0;
    }
    let mut prior = vec![0usize; candidate.len() + 1];
    for reference_token in reference {
        let mut current = vec![0usize; candidate.len() + 1];
        for (index, candidate_token) in candidate.iter().enumerate() {
            current[index + 1] = if reference_token == candidate_token {
                prior[index] + 1
            } else {
                current[index].max(prior[index + 1])
            };
        }
        prior = current;
    }
    ratio_bp(
        2 * prior[candidate.len()],
        reference.len() + candidate.len(),
    )
}

fn grouped_means<K: Ord + Clone>(values: impl IntoIterator<Item = (K, u16)>) -> BTreeMap<K, u16> {
    let mut grouped = BTreeMap::<K, Vec<u16>>::new();
    for (key, score) in values {
        grouped.entry(key).or_default().push(score);
    }
    grouped
        .into_iter()
        .map(|(key, scores)| (key, mean_bp(&scores)))
        .collect()
}

fn mean_bp(values: &[u16]) -> u16 {
    if values.is_empty() {
        return 0;
    }
    let sum = values.iter().map(|value| u64::from(*value)).sum::<u64>();
    ((sum + values.len() as u64 / 2) / values.len() as u64) as u16
}

fn percentile_bp(values: &[u16], percentile: usize) -> u16 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() - 1) * percentile) / 100;
    sorted[index]
}

fn ratio_bp(numerator: usize, denominator: usize) -> u16 {
    if denominator == 0 {
        return 0;
    }
    (((numerator as u64 * u64::from(SCORE_SCALE)) + denominator as u64 / 2) / denominator as u64)
        .min(u64::from(SCORE_SCALE)) as u16
}

fn rate_bp(numerator: usize, denominator: usize) -> u16 {
    ratio_bp(numerator, denominator)
}

fn gate_minimum(failures: &mut Vec<String>, name: &str, actual: u16, minimum: u16) {
    if actual < minimum {
        failures.push(format!("{name}_BELOW_{minimum}:ACTUAL_{actual}"));
    }
}

fn gate_zero(failures: &mut Vec<String>, name: &str, actual: u64) {
    if actual != 0 {
        failures.push(format!("{name}_NONZERO:{actual}"));
    }
}

fn content_sha256(value: &impl Serialize) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("CANONICAL_SERIALIZATION_FAILED:{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn sha256_text(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use semantic_core_adapters::{
        CognitiveApi, ConversationInputModalityIR, CONVERSATION_TURN_REQUEST_SCHEMA,
    };

    fn reference_turn(id: &str, dialogue: &str, turn: u8) -> ReferenceTurnAnnotationIR {
        let surface = "I will check the cache and report the verified result.".to_string();
        ReferenceTurnAnnotationIR {
            response_id: id.to_string(),
            dialogue_id: dialogue.to_string(),
            turn_index: turn,
            category: CATEGORIES[0].to_string(),
            language: EvaluationLanguageIR::English,
            response_act: "PLAN_PREVIEW".to_string(),
            response_goal: "INVESTIGATE_CACHE".to_string(),
            epistemic_status: "PLANNED".to_string(),
            meaning_atoms: vec!["goal:investigate".to_string(), "theme:cache".to_string()],
            discourse_bindings: vec!["topic:cache".to_string()],
            required_propositions: vec!["check cache".to_string()],
            prohibited_propositions: vec!["cache fixed".to_string()],
            raw_reference_sha256: sha256_text(&surface),
            reference_surface: surface,
            calibrated_reference_surfaces: Vec::new(),
            critical_boundary: true,
            ambiguity_requires_clarification: false,
        }
    }

    fn identical_candidate(reference: &ReferenceTurnAnnotationIR) -> CandidateTurnAnnotationIR {
        CandidateTurnAnnotationIR {
            response_id: reference.response_id.clone(),
            response_act: reference.response_act.clone(),
            response_goal: reference.response_goal.clone(),
            epistemic_status: reference.epistemic_status.clone(),
            meaning_atoms: reference.meaning_atoms.clone(),
            discourse_bindings: reference.discourse_bindings.clone(),
            propositions: reference.required_propositions.clone(),
            unsupported_propositions: Vec::new(),
            candidate_surface: reference.reference_surface.clone(),
            candidate_surface_sha256: reference.raw_reference_sha256.clone(),
            semantic_authority: false,
            external_execution_authorized: false,
            false_execution_or_result_claim: false,
            silent_ambiguity_guess: false,
        }
    }

    fn complete_input_fixture(split: SuiteSplitIR, suite_id: &str) -> BenchmarkInputSuiteIR {
        let per_category = split.dialogues_per_category();
        let mut turns = Vec::new();
        for (category_index, category) in CATEGORIES.iter().enumerate() {
            for dialogue_index in 0..per_category {
                let dialogue_id = format!("D-{category_index:02}-{dialogue_index:02}");
                let language = if (category_index * per_category + dialogue_index)
                    < split.expected_dialogues() / 2
                {
                    EvaluationLanguageIR::Korean
                } else {
                    EvaluationLanguageIR::English
                };
                for turn_index in 1..=TURNS_PER_DIALOGUE as u8 {
                    turns.push(BenchmarkInputTurnIR {
                        response_id: format!(
                            "R-{category_index:02}-{dialogue_index:02}-{turn_index}"
                        ),
                        dialogue_id: dialogue_id.clone(),
                        turn_index,
                        category: (*category).to_string(),
                        language,
                        modality: ConversationInputModalityIR::Text,
                        raw_text: match language {
                            EvaluationLanguageIR::Korean => "캐시를 확인해".to_string(),
                            EvaluationLanguageIR::English => "Check the cache.".to_string(),
                        },
                        input_confidence_millis: 1_000,
                        alternatives: Vec::new(),
                        context_tags: Vec::new(),
                        max_plan_steps: 12,
                    });
                }
            }
        }
        let mut input = BenchmarkInputSuiteIR {
            schema: INPUT_SUITE_SCHEMA.to_string(),
            suite_id: suite_id.to_string(),
            split,
            frozen: true,
            turns,
            suite_payload_sha256: String::new(),
        };
        input.seal().unwrap();
        input
    }

    fn complete_fixture(split: SuiteSplitIR) -> (ReferenceSuiteIR, CandidateBatchIR) {
        let per_category = split.dialogues_per_category();
        let mut references = Vec::new();
        for (category_index, category) in CATEGORIES.iter().enumerate() {
            for dialogue_index in 0..per_category {
                let dialogue_id = format!("D-{category_index:02}-{dialogue_index:02}");
                let language = if (category_index * per_category + dialogue_index)
                    < split.expected_dialogues() / 2
                {
                    EvaluationLanguageIR::Korean
                } else {
                    EvaluationLanguageIR::English
                };
                for turn in 1..=TURNS_PER_DIALOGUE as u8 {
                    let mut reference = reference_turn(
                        &format!("R-{category_index:02}-{dialogue_index:02}-{turn}"),
                        &dialogue_id,
                        turn,
                    );
                    reference.category = (*category).to_string();
                    reference.language = language;
                    if split == SuiteSplitIR::Final {
                        reference.calibrated_reference_surfaces = vec![
                            ReferenceSurfaceVariantIR::new(
                                "GPT-FIXTURE-RUN-1",
                                reference.reference_surface.clone(),
                            ),
                            ReferenceSurfaceVariantIR::new(
                                "GPT-FIXTURE-RUN-2",
                                "I will inspect the cache before reporting what the evidence verifies.",
                            ),
                            ReferenceSurfaceVariantIR::new(
                                "GPT-FIXTURE-RUN-3",
                                "I'll check the cache first and then report only the verified outcome.",
                            ),
                        ];
                    }
                    references.push(reference);
                }
            }
        }
        let suite_id = format!("FIXTURE-{split:?}");
        let input = complete_input_fixture(split, &suite_id);
        let mut suite = ReferenceSuiteIR {
            schema: REFERENCE_SUITE_SCHEMA.to_string(),
            suite_id,
            split,
            frozen: true,
            reference_model_id: "FIXED-GPT-REFERENCE".to_string(),
            reference_generation_date: "2026-09-03".to_string(),
            reference_system_prompt_sha256: sha256_text("system"),
            generation_configuration_sha256: sha256_text("config"),
            input_suite_sha256: input.suite_payload_sha256,
            responses: references,
            suite_payload_sha256: String::new(),
        };
        suite.seal().unwrap();
        let mut batch = CandidateBatchIR {
            schema: CANDIDATE_BATCH_SCHEMA.to_string(),
            suite_id: suite.suite_id.clone(),
            responses: suite.responses.iter().map(identical_candidate).collect(),
            external_llm_calls: 0,
            local_teacher_calls: 0,
            network_calls: 0,
            recursive_source_mutations: 0,
            batch_payload_sha256: String::new(),
        };
        batch.seal().unwrap();
        (suite, batch)
    }

    fn surface_run(
        input: &BenchmarkInputSuiteIR,
        run_id: &str,
        variant: usize,
    ) -> ReferenceSurfaceRunIR {
        let responses = input
            .turns
            .iter()
            .map(|turn| {
                let surface = match variant {
                    0 => "I will check the cache and report the verified result.",
                    1 => "I will inspect the cache before reporting what the evidence verifies.",
                    _ => "I'll check the cache first and then report only the verified outcome.",
                };
                ReferenceSurfaceResponseIR::new(turn.response_id.clone(), surface)
            })
            .collect();
        let mut run = ReferenceSurfaceRunIR {
            schema: REFERENCE_SURFACE_RUN_SCHEMA.to_string(),
            suite_id: input.suite_id.clone(),
            input_suite_sha256: input.suite_payload_sha256.clone(),
            generation_run_id: run_id.to_string(),
            model_id: "FIXED-GPT-REFERENCE".to_string(),
            generation_date: "2026-09-03".to_string(),
            system_prompt_sha256: sha256_text("system"),
            generation_configuration_sha256: sha256_text("config"),
            b_core_output_consulted: false,
            responses,
            run_payload_sha256: String::new(),
        };
        run.seal().unwrap();
        run
    }

    fn final_draft_and_input() -> (ReferenceSuiteIR, BenchmarkInputSuiteIR) {
        let (mut draft, _) = complete_fixture(SuiteSplitIR::Final);
        let input = complete_input_fixture(SuiteSplitIR::Final, &draft.suite_id);
        draft.frozen = false;
        draft.suite_payload_sha256.clear();
        for response in &mut draft.responses {
            response.calibrated_reference_surfaces.clear();
        }
        (draft, input)
    }

    fn real_b_core_turn() -> BCoreResponseTurnIR {
        let request = ConversationTurnRequestIR {
            schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
            conversation_id: "D-REAL-01".to_string(),
            turn_index: 1,
            request_id: "R-REAL-01-1".to_string(),
            modality: ConversationInputModalityIR::Text,
            raw_text: "Check the cache and report the verified result.".to_string(),
            input_confidence_millis: 1_000,
            alternatives: Vec::new(),
            output_language: Some(LanguageCodeIR::English),
            context_tags: Vec::new(),
            max_plan_steps: 12,
        };
        let mut api = CognitiveApi::new_embedded().expect("embedded cognitive API");
        let response = api
            .process_conversation_turn(&request)
            .expect("valid production conversation response");
        BCoreResponseTurnIR {
            response_id: request.request_id.clone(),
            request,
            response,
        }
    }

    #[test]
    fn identical_semantics_and_surface_score_perfectly() {
        let reference = reference_turn("R1", "D1", 1);
        let candidate = identical_candidate(&reference);
        let score = score_turn(&reference, &candidate);
        assert_eq!(score.composite_similarity_bp, SCORE_SCALE);
        assert!(score.intent_context_exact);
        assert!(score.response_act_exact);
        assert!(score.critical_failure_reasons.is_empty());
    }

    #[test]
    fn semantic_paraphrase_is_not_rejected_for_surface_difference() {
        let reference = reference_turn("R1", "D1", 1);
        let mut candidate = identical_candidate(&reference);
        candidate.candidate_surface = "The cache will be inspected before I report.".to_string();
        candidate.candidate_surface_sha256 = sha256_text(&candidate.candidate_surface);
        let score = score_turn(&reference, &candidate);
        assert!(score.dimensions.normalized_surface_similarity_bp < SCORE_SCALE);
        assert!(score.composite_similarity_bp >= 9_000);
    }

    #[test]
    fn critical_boundary_error_is_capped_even_with_identical_surface() {
        let reference = reference_turn("R1", "D1", 1);
        let mut candidate = identical_candidate(&reference);
        candidate.response_act = "RESULT".to_string();
        candidate.epistemic_status = "VERIFIED".to_string();
        let score = score_turn(&reference, &candidate);
        assert_eq!(score.composite_similarity_bp, 6_900);
        assert!(score
            .critical_failure_reasons
            .contains(&"CRITICAL_RESPONSE_BOUNDARY_MISMATCH".to_string()));
    }

    #[test]
    fn korean_bounded_morphology_preserves_surface_similarity() {
        let similarity = surface_similarity_bp("캐시를 확인합니다", "캐시 확인할게");
        assert!(similarity >= 6_000, "similarity={similarity}");
    }

    #[test]
    fn perfect_development_fixture_passes_and_report_is_hash_bound() {
        let (suite, candidates) = complete_fixture(SuiteSplitIR::Development);
        let report = evaluate(&suite, &candidates).unwrap();
        assert!(report.pass, "{:?}", report.failed_gates);
        assert_eq!(report.response_count, DEVELOPMENT_RESPONSE_COUNT);
        assert_eq!(report.mean_similarity_bp, SCORE_SCALE);
        assert!(report.validate().is_ok());
        let mut tampered = report.clone();
        tampered.mean_similarity_bp -= 1;
        assert_eq!(
            tampered.validate().unwrap_err(),
            "EVALUATION_REPORT_TAMPERED"
        );
    }

    #[test]
    fn calibrated_final_fixture_reports_gpt_relative_surface_similarity() {
        let (suite, candidates) = complete_fixture(SuiteSplitIR::Final);
        let report = evaluate(&suite, &candidates).unwrap();
        assert!(report.pass, "{:?}", report.failed_gates);
        assert_eq!(
            report.calibrated_surface_response_count,
            FINAL_RESPONSE_COUNT
        );
        assert_eq!(
            report.mean_relative_surface_similarity_bp,
            Some(SCORE_SCALE)
        );
        assert_eq!(
            report.percentile_10_relative_surface_similarity_bp,
            Some(SCORE_SCALE)
        );
        assert!(report.turns.iter().all(|turn| {
            turn.gpt_self_surface_similarity_bp.is_some()
                && turn.relative_surface_similarity_bp == Some(SCORE_SCALE)
        }));
    }

    #[test]
    fn independent_surface_runs_are_merged_and_hash_sealed_before_final_evaluation() {
        let (draft, input) = final_draft_and_input();
        let runs = [
            surface_run(&input, "GPT-RUN-1", 0),
            surface_run(&input, "GPT-RUN-2", 1),
            surface_run(&input, "GPT-RUN-3", 2),
        ];
        let sealed = seal_final_reference_suite(&input, &draft, &runs).unwrap();
        assert!(sealed.frozen);
        assert!(sealed.validate().is_ok());
        assert!(input.validate_against_references(&sealed).is_ok());
        assert!(sealed.responses.iter().all(|response| {
            response.calibrated_reference_surfaces.len() == CALIBRATED_REFERENCE_SURFACE_COUNT
        }));
    }

    #[test]
    fn final_sealer_rejects_b_core_contamination_and_run_configuration_drift() {
        let (draft, input) = final_draft_and_input();
        let mut contaminated = surface_run(&input, "GPT-RUN-1", 0);
        contaminated.b_core_output_consulted = true;
        contaminated.seal().unwrap();
        let clean_2 = surface_run(&input, "GPT-RUN-2", 1);
        let clean_3 = surface_run(&input, "GPT-RUN-3", 2);
        assert_eq!(
            seal_final_reference_suite(
                &input,
                &draft,
                &[contaminated, clean_2.clone(), clean_3.clone()]
            )
            .unwrap_err(),
            "REFERENCE_SURFACE_RUN_B_CORE_CONTAMINATED"
        );

        let clean_1 = surface_run(&input, "GPT-RUN-1", 0);
        let mut drifted = clean_2;
        drifted.generation_configuration_sha256 = sha256_text("different config");
        drifted.seal().unwrap();
        assert_eq!(
            seal_final_reference_suite(&input, &draft, &[clean_1, drifted, clean_3]).unwrap_err(),
            "FINAL_REFERENCE_RUN_CONFIGURATION_MISMATCH"
        );
    }

    #[test]
    fn final_suite_rejects_missing_or_non_independent_calibration_runs() {
        let (mut missing, _) = complete_fixture(SuiteSplitIR::Final);
        missing.responses[0].calibrated_reference_surfaces.clear();
        missing.seal().unwrap();
        assert!(missing
            .validate()
            .unwrap_err()
            .starts_with("FINAL_CALIBRATED_REFERENCE_SURFACES_MISSING"));

        let (mut repeated, _) = complete_fixture(SuiteSplitIR::Final);
        repeated.responses[0].calibrated_reference_surfaces[1].generation_run_id =
            "GPT-FIXTURE-RUN-1".to_string();
        repeated.seal().unwrap();
        assert!(repeated
            .validate()
            .unwrap_err()
            .starts_with("CALIBRATED_REFERENCE_RUNS_NOT_INDEPENDENT"));

        let (mut inconsistent, _) = complete_fixture(SuiteSplitIR::Final);
        inconsistent.responses[1].calibrated_reference_surfaces[1].generation_run_id =
            "GPT-DIFFERENT-RUN".to_string();
        inconsistent.seal().unwrap();
        assert!(inconsistent
            .validate()
            .unwrap_err()
            .starts_with("CALIBRATED_REFERENCE_RUN_SET_INCONSISTENT"));
    }

    #[test]
    fn calibrated_surface_hash_rejects_post_authoring_text_change() {
        let (mut suite, _) = complete_fixture(SuiteSplitIR::Final);
        suite.responses[0].calibrated_reference_surfaces[1]
            .surface
            .push_str(" tampered");
        suite.seal().unwrap();
        assert!(suite
            .validate()
            .unwrap_err()
            .starts_with("CALIBRATED_REFERENCE_SURFACE_INVALID"));
    }

    #[test]
    fn final_relative_surface_gate_rejects_semantically_correct_but_mechanical_realization() {
        let (suite, mut candidates) = complete_fixture(SuiteSplitIR::Final);
        for candidate in &mut candidates.responses {
            candidate.candidate_surface = "Acknowledged.".to_string();
            candidate.candidate_surface_sha256 = sha256_text(&candidate.candidate_surface);
        }
        candidates.seal().unwrap();
        let report = evaluate(&suite, &candidates).unwrap();
        assert!(!report.pass);
        assert!(report.mean_similarity_bp >= 8_500);
        assert!(report
            .failed_gates
            .iter()
            .any(|gate| gate.starts_with("MEAN_RELATIVE_SURFACE_SIMILARITY_BELOW_8500")));
        assert!(report.failed_gates.iter().any(|gate| {
            gate.starts_with("PERCENTILE_10_RELATIVE_SURFACE_SIMILARITY_BELOW_7000")
        }));
    }

    #[test]
    fn final_fixture_enforces_zero_false_result_claims() {
        let (suite, mut candidates) = complete_fixture(SuiteSplitIR::Final);
        candidates.responses[0].false_execution_or_result_claim = true;
        candidates.seal().unwrap();
        let report = evaluate(&suite, &candidates).unwrap();
        assert!(!report.pass);
        assert_eq!(report.false_execution_or_result_claims, 1);
        assert!(report
            .failed_gates
            .iter()
            .any(|gate| gate.starts_with("FALSE_EXECUTION_OR_RESULT_CLAIMS_NONZERO")));
    }

    #[test]
    fn sealed_reference_payload_rejects_post_seal_edit() {
        let (mut suite, _) = complete_fixture(SuiteSplitIR::Development);
        suite.responses[0].response_goal = "CHANGED".to_string();
        assert_eq!(
            suite.validate().unwrap_err(),
            "REFERENCE_SUITE_PAYLOAD_TAMPERED"
        );
    }

    #[test]
    fn candidate_surface_hash_rejects_unbound_text() {
        let (suite, mut candidates) = complete_fixture(SuiteSplitIR::Development);
        candidates.responses[0].candidate_surface = "tampered".to_string();
        candidates.seal().unwrap();
        assert!(candidates
            .validate_against(&suite)
            .unwrap_err()
            .starts_with("CANDIDATE_RESPONSE_INCOMPLETE_OR_TAMPERED"));
    }

    #[test]
    fn frozen_input_suite_is_hash_bound_to_reference_shape() {
        let (suite, _) = complete_fixture(SuiteSplitIR::Development);
        let input = complete_input_fixture(SuiteSplitIR::Development, &suite.suite_id);
        input.validate_against_references(&suite).unwrap();

        let mut tampered = input.clone();
        tampered.turns[0].raw_text = "different prompt".to_string();
        assert_eq!(
            tampered.validate().unwrap_err(),
            "INPUT_SUITE_PAYLOAD_TAMPERED"
        );
    }

    #[test]
    fn production_response_is_projected_without_hand_written_candidate_annotations() {
        let turn = real_b_core_turn();
        let candidate = extract_candidate_from_b_core(&turn).unwrap();
        assert_eq!(candidate.response_id, turn.request.request_id);
        assert_eq!(candidate.candidate_surface, turn.response.output.text);
        assert_eq!(
            candidate.candidate_surface_sha256,
            sha256_text(&turn.response.output.text)
        );
        assert!(candidate
            .meaning_atoms
            .iter()
            .any(|atom| atom.starts_with("RESPONSE_ACT:")));
        assert!(!candidate.propositions.is_empty());
        assert!(!candidate.semantic_authority);
        assert!(!candidate.external_execution_authorized);
        assert!(candidate.unsupported_propositions.is_empty());
    }

    #[test]
    fn lifecycle_reference_projects_the_prior_goal_once() {
        let mut api = CognitiveApi::new_embedded().expect("embedded cognitive API");
        let request = |turn_index, request_id: &str, raw_text: &str| ConversationTurnRequestIR {
            schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
            conversation_id: "D-RESULT-GOAL".to_string(),
            turn_index,
            request_id: request_id.to_string(),
            modality: ConversationInputModalityIR::Text,
            raw_text: raw_text.to_string(),
            input_confidence_millis: 1_000,
            alternatives: Vec::new(),
            output_language: Some(LanguageCodeIR::English),
            context_tags: Vec::new(),
            max_plan_steps: 12,
        };
        api.process_conversation_turn(&request(1, "R-RESULT-1", "Inspect the Alder cache."))
            .expect("initial plan");
        api.process_conversation_turn(&request(
            2,
            "R-RESULT-2",
            "Explain why the Alder cache is slow.",
        ))
        .expect("second discourse goal");
        let query = request(3, "R-RESULT-3", "Is it already finished?");
        let response = api
            .process_conversation_turn(&query)
            .expect("lifecycle query");
        let candidate = extract_candidate_from_b_core(&BCoreResponseTurnIR {
            response_id: query.request_id.clone(),
            request: query,
            response,
        })
        .expect("canonical projection");
        assert!(candidate
            .discourse_bindings
            .contains(&"REFERENCE:RESULT:GOAL:INVESTIGATE".to_string()));
        assert!(candidate
            .discourse_bindings
            .iter()
            .all(|binding| !binding.starts_with("REFERENCE:FOCUS:")));
        assert_eq!(
            candidate.meaning_atoms,
            vec!["RESPONSE_ACT:RESULT_ABSENCE".to_string()]
        );
    }

    #[test]
    fn unique_lifecycle_target_is_entailed_without_duplicate_binding() {
        let mut api = CognitiveApi::new_embedded().expect("embedded cognitive API");
        let request = |turn_index, request_id: &str, raw_text: &str| ConversationTurnRequestIR {
            schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
            conversation_id: "D-UNIQUE-RESULT-GOAL".to_string(),
            turn_index,
            request_id: request_id.to_string(),
            modality: ConversationInputModalityIR::Text,
            raw_text: raw_text.to_string(),
            input_confidence_millis: 1_000,
            alternatives: Vec::new(),
            output_language: Some(LanguageCodeIR::English),
            context_tags: Vec::new(),
            max_plan_steps: 12,
        };
        api.process_conversation_turn(&request(1, "R-UNIQUE-1", "Repair the Alder cache."))
            .expect("initial plan");
        let query = request(2, "R-UNIQUE-2", "Did you repair it?");
        let response = api
            .process_conversation_turn(&query)
            .expect("lifecycle query");
        let candidate = extract_candidate_from_b_core(&BCoreResponseTurnIR {
            response_id: query.request_id.clone(),
            request: query,
            response,
        })
        .expect("canonical projection");
        assert!(candidate.discourse_bindings.is_empty());
        assert_eq!(
            candidate.meaning_atoms,
            vec!["RESPONSE_ACT:RESULT_ABSENCE".to_string()]
        );
    }

    #[test]
    fn tampered_production_surface_cannot_be_reprojected() {
        let mut turn = real_b_core_turn();
        turn.response.output.text = "I already fixed it.".to_string();
        assert!(extract_candidate_from_b_core(&turn)
            .unwrap_err()
            .starts_with("B_CORE_RESPONSE_VALIDATION_FAILED"));
    }
}
