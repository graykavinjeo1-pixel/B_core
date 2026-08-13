use dockable_semantic_core::{
    DockableCore, ExperienceError, ExperienceIR, ExperienceInjectionReceiptIR,
    ExperienceSnapshotIR, PlanGoalIR, PlanIR, PlanOperationIR, PlanningError, PLAN_GOAL_SCHEMA,
};
use serde::{Deserialize, Serialize};

use crate::language_knowledge::{
    LanguageCodeIR, LanguageKnowledgeBase, LanguageKnowledgeEntryIR, LanguageKnowledgeError,
    LanguageKnowledgeStatisticsIR, LanguageUnderstandingIR,
};

pub const NATURAL_LANGUAGE_REQUEST_SCHEMA: &str = "B_CORE_NATURAL_LANGUAGE_REQUEST_1";
pub const NATURAL_LANGUAGE_RESPONSE_SCHEMA: &str = "B_CORE_NATURAL_LANGUAGE_RESPONSE_1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalLanguageRequestIR {
    pub schema: String,
    pub request_id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_language: Option<LanguageCodeIR>,
    #[serde(default)]
    pub context_tags: Vec<String>,
    pub max_plan_steps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalLanguageOutputIR {
    pub language: LanguageCodeIR,
    pub text: String,
    pub grounded_plan_sha256: String,
    pub unsupported_freeform_claims: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalLanguageResponseIR {
    pub schema: String,
    pub request_id: String,
    pub understanding: LanguageUnderstandingIR,
    pub plan: PlanIR,
    pub output: NaturalLanguageOutputIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CognitiveApiCommandIR {
    InjectExperience { experience: ExperienceIR },
    ExportExperienceSnapshot,
    ImportExperienceSnapshot { snapshot: ExperienceSnapshotIR },
    InjectLanguageKnowledge { entry: LanguageKnowledgeEntryIR },
    ProcessNaturalLanguage { request: NaturalLanguageRequestIR },
    LanguageKnowledgeStatistics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CognitiveApiPayloadIR {
    ExperienceInjectionReceipt(ExperienceInjectionReceiptIR),
    ExperienceInjectionReceipts(Vec<ExperienceInjectionReceiptIR>),
    ExperienceSnapshot(ExperienceSnapshotIR),
    LanguageKnowledgeInserted(bool),
    NaturalLanguageResponse(Box<NaturalLanguageResponseIR>),
    LanguageKnowledgeStatistics(LanguageKnowledgeStatisticsIR),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveApiResponseIR {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<CognitiveApiPayloadIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<CognitiveApiError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CognitiveApiError {
    CoreLoad,
    InvalidRequest,
    LanguageKnowledge,
    Experience,
    Planning,
    JsonInput,
    JsonOutput,
}

/// Local, deterministic public API for natural-language planning and bounded
/// experience injection. Language interpretation proposes typed IR; the core
/// planner remains the plan authority and every output sentence is rendered
/// from that validated IR.
pub struct CognitiveApi {
    core: DockableCore,
    language_knowledge: LanguageKnowledgeBase,
}

impl CognitiveApi {
    pub fn new_embedded() -> Result<Self, CognitiveApiError> {
        Ok(Self {
            core: DockableCore::load_embedded().map_err(|_| CognitiveApiError::CoreLoad)?,
            language_knowledge: LanguageKnowledgeBase::default(),
        })
    }

    pub fn inject_experience(
        &mut self,
        experience: ExperienceIR,
    ) -> Result<ExperienceInjectionReceiptIR, CognitiveApiError> {
        self.core
            .inject_experience(experience)
            .map_err(map_experience_error)
    }

    pub fn inject_experience_json(&mut self, json: &str) -> Result<String, CognitiveApiError> {
        let experience =
            serde_json::from_str::<ExperienceIR>(json).map_err(|_| CognitiveApiError::JsonInput)?;
        serde_json::to_string(&self.inject_experience(experience)?)
            .map_err(|_| CognitiveApiError::JsonOutput)
    }

    pub fn export_experience_snapshot_json(&self) -> Result<String, CognitiveApiError> {
        serde_json::to_string(&self.core.export_experience_snapshot())
            .map_err(|_| CognitiveApiError::JsonOutput)
    }

    pub fn import_experience_snapshot_json(
        &mut self,
        json: &str,
    ) -> Result<String, CognitiveApiError> {
        let snapshot = serde_json::from_str::<ExperienceSnapshotIR>(json)
            .map_err(|_| CognitiveApiError::JsonInput)?;
        serde_json::to_string(
            &self
                .core
                .import_experience_snapshot(&snapshot)
                .map_err(map_experience_error)?,
        )
        .map_err(|_| CognitiveApiError::JsonOutput)
    }

    pub fn inject_language_knowledge(
        &mut self,
        entry: LanguageKnowledgeEntryIR,
    ) -> Result<bool, CognitiveApiError> {
        self.language_knowledge
            .inject(entry)
            .map_err(map_language_error)
    }

    pub fn process(
        &self,
        request: &NaturalLanguageRequestIR,
    ) -> Result<NaturalLanguageResponseIR, CognitiveApiError> {
        validate_request(request)?;
        let mut understanding = self
            .language_knowledge
            .understand(&request.text)
            .map_err(map_language_error)?;
        understanding
            .semantic_tags
            .extend(request.context_tags.iter().cloned());
        understanding.semantic_tags.sort();
        understanding.semantic_tags.dedup();
        let plan = self
            .core
            .generate_plan(&PlanGoalIR {
                schema: PLAN_GOAL_SCHEMA.to_string(),
                goal_id: request.request_id.clone(),
                intent: understanding.intent,
                subject: understanding.subject.clone(),
                constraints: understanding.constraints.clone(),
                desired_outcomes: understanding.desired_outcomes.clone(),
                context_tags: understanding.semantic_tags.clone(),
                max_steps: request.max_plan_steps,
            })
            .map_err(map_planning_error)?;
        let output_language = request
            .output_language
            .filter(|language| matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::English))
            .unwrap_or(match understanding.detected_language {
                LanguageCodeIR::Korean | LanguageCodeIR::Mixed => LanguageCodeIR::Korean,
                _ => LanguageCodeIR::English,
            });
        let output = render_plan(output_language, &understanding, &plan);
        Ok(NaturalLanguageResponseIR {
            schema: NATURAL_LANGUAGE_RESPONSE_SCHEMA.to_string(),
            request_id: request.request_id.clone(),
            understanding,
            plan,
            output,
        })
    }

    pub fn process_json(&self, json: &str) -> Result<String, CognitiveApiError> {
        let request = serde_json::from_str::<NaturalLanguageRequestIR>(json)
            .map_err(|_| CognitiveApiError::JsonInput)?;
        serde_json::to_string(&self.process(&request)?).map_err(|_| CognitiveApiError::JsonOutput)
    }

    pub fn language_knowledge_statistics(&self) -> LanguageKnowledgeStatisticsIR {
        self.language_knowledge.statistics()
    }

    pub fn retained_experience_count(&self) -> usize {
        self.core.retained_experience_count()
    }

    pub fn execute_command(&mut self, command: CognitiveApiCommandIR) -> CognitiveApiResponseIR {
        let result = match command {
            CognitiveApiCommandIR::InjectExperience { experience } => self
                .inject_experience(experience)
                .map(CognitiveApiPayloadIR::ExperienceInjectionReceipt),
            CognitiveApiCommandIR::ExportExperienceSnapshot => Ok(
                CognitiveApiPayloadIR::ExperienceSnapshot(self.core.export_experience_snapshot()),
            ),
            CognitiveApiCommandIR::ImportExperienceSnapshot { snapshot } => self
                .core
                .import_experience_snapshot(&snapshot)
                .map(CognitiveApiPayloadIR::ExperienceInjectionReceipts)
                .map_err(map_experience_error),
            CognitiveApiCommandIR::InjectLanguageKnowledge { entry } => self
                .inject_language_knowledge(entry)
                .map(CognitiveApiPayloadIR::LanguageKnowledgeInserted),
            CognitiveApiCommandIR::ProcessNaturalLanguage { request } => self
                .process(&request)
                .map(|response| CognitiveApiPayloadIR::NaturalLanguageResponse(Box::new(response))),
            CognitiveApiCommandIR::LanguageKnowledgeStatistics => {
                Ok(CognitiveApiPayloadIR::LanguageKnowledgeStatistics(
                    self.language_knowledge_statistics(),
                ))
            }
        };
        match result {
            Ok(payload) => CognitiveApiResponseIR {
                ok: true,
                payload: Some(payload),
                error: None,
            },
            Err(error) => CognitiveApiResponseIR {
                ok: false,
                payload: None,
                error: Some(error),
            },
        }
    }

    pub fn execute_command_json(&mut self, json: &str) -> Result<String, CognitiveApiError> {
        let command = serde_json::from_str::<CognitiveApiCommandIR>(json)
            .map_err(|_| CognitiveApiError::JsonInput)?;
        serde_json::to_string(&self.execute_command(command))
            .map_err(|_| CognitiveApiError::JsonOutput)
    }
}

fn validate_request(request: &NaturalLanguageRequestIR) -> Result<(), CognitiveApiError> {
    if request.schema != NATURAL_LANGUAGE_REQUEST_SCHEMA
        || request.request_id.trim().is_empty()
        || request.request_id.len() > 128
        || request.text.trim().is_empty()
        || request.text.len() > 64 * 1024
        || !(5..=32).contains(&request.max_plan_steps)
        || request.context_tags.len() > 64
        || request
            .context_tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 128)
    {
        return Err(CognitiveApiError::InvalidRequest);
    }
    Ok(())
}

fn map_experience_error(_: ExperienceError) -> CognitiveApiError {
    CognitiveApiError::Experience
}

fn map_language_error(_: LanguageKnowledgeError) -> CognitiveApiError {
    CognitiveApiError::LanguageKnowledge
}

fn map_planning_error(_: PlanningError) -> CognitiveApiError {
    CognitiveApiError::Planning
}

fn render_plan(
    language: LanguageCodeIR,
    understanding: &LanguageUnderstandingIR,
    plan: &PlanIR,
) -> NaturalLanguageOutputIR {
    let text = match language {
        LanguageCodeIR::Korean => render_korean(understanding, plan),
        _ => render_english(understanding, plan),
    };
    NaturalLanguageOutputIR {
        language,
        text,
        grounded_plan_sha256: plan.plan_sha256.clone(),
        unsupported_freeform_claims: 0,
    }
}

fn render_korean(understanding: &LanguageUnderstandingIR, plan: &PlanIR) -> String {
    let mut lines = vec![format!(
        "요청을 '{}' 의도로 해석했습니다. 대상: {}",
        korean_intent(understanding.intent),
        understanding.subject
    )];
    if !plan.recalled_experiences.is_empty() {
        lines.push(format!(
            "관련 성공 경험 {}건을 계획 근거로 사용합니다.",
            plan.recalled_experiences.len()
        ));
    }
    for (index, step) in plan.steps.iter().enumerate() {
        lines.push(format!(
            "{}. {}",
            index + 1,
            korean_operation(step.operation)
        ));
    }
    lines.push(format!(
        "검증 단계: {} / 계획 해시: {}",
        plan.terminal_verification_step_id, plan.plan_sha256
    ));
    lines.join("\n")
}

fn render_english(understanding: &LanguageUnderstandingIR, plan: &PlanIR) -> String {
    let mut lines = vec![format!(
        "I interpreted the request as '{}'. Subject: {}",
        english_intent(understanding.intent),
        understanding.subject
    )];
    if !plan.recalled_experiences.is_empty() {
        lines.push(format!(
            "The plan uses {} relevant successful experience(s).",
            plan.recalled_experiences.len()
        ));
    }
    for (index, step) in plan.steps.iter().enumerate() {
        lines.push(format!(
            "{}. {}",
            index + 1,
            english_operation(step.operation)
        ));
    }
    lines.push(format!(
        "Verification step: {} / plan hash: {}",
        plan.terminal_verification_step_id, plan.plan_sha256
    ));
    lines.join("\n")
}

fn korean_intent(intent: dockable_semantic_core::PlanIntentIR) -> &'static str {
    match intent {
        dockable_semantic_core::PlanIntentIR::Plan => "계획 생성",
        dockable_semantic_core::PlanIntentIR::Investigate => "조사·진단",
        dockable_semantic_core::PlanIntentIR::Repair => "수리",
        dockable_semantic_core::PlanIntentIR::Create => "생성·구현",
        dockable_semantic_core::PlanIntentIR::Learn => "학습",
        dockable_semantic_core::PlanIntentIR::Explain => "설명",
        dockable_semantic_core::PlanIntentIR::Communicate => "전달",
        dockable_semantic_core::PlanIntentIR::Execute => "실행",
    }
}

fn english_intent(intent: dockable_semantic_core::PlanIntentIR) -> &'static str {
    match intent {
        dockable_semantic_core::PlanIntentIR::Plan => "planning",
        dockable_semantic_core::PlanIntentIR::Investigate => "investigation",
        dockable_semantic_core::PlanIntentIR::Repair => "repair",
        dockable_semantic_core::PlanIntentIR::Create => "creation",
        dockable_semantic_core::PlanIntentIR::Learn => "learning",
        dockable_semantic_core::PlanIntentIR::Explain => "explanation",
        dockable_semantic_core::PlanIntentIR::Communicate => "communication",
        dockable_semantic_core::PlanIntentIR::Execute => "execution",
    }
}

fn korean_operation(operation: PlanOperationIR) -> &'static str {
    match operation {
        PlanOperationIR::ObserveCurrentState => "현재 상태 관찰",
        PlanOperationIR::RecallRelevantExperience => "관련 경험 회상",
        PlanOperationIR::DerivePostconditions => "완료 조건 도출",
        PlanOperationIR::ModelKnowledgeGap => "지식 공백 모델링",
        PlanOperationIR::GenerateCandidates => "후보 생성",
        PlanOperationIR::PredictConsequences => "결과 예측",
        PlanOperationIR::RunDiagnostic => "진단 실행",
        PlanOperationIR::ValidateCandidates => "후보 검증",
        PlanOperationIR::ApplySelectedAction => "선택 행동 적용",
        PlanOperationIR::VerifyOutcome => "결과 검증",
        PlanOperationIR::GeneralizeLesson => "교훈 일반화",
        PlanOperationIR::StoreSuccessfulExperience => "성공 경험 저장",
        PlanOperationIR::SynthesizeExplanation => "설명 합성",
        PlanOperationIR::CommunicateResult => "결과 전달",
    }
}

fn english_operation(operation: PlanOperationIR) -> &'static str {
    match operation {
        PlanOperationIR::ObserveCurrentState => "Observe current state",
        PlanOperationIR::RecallRelevantExperience => "Recall relevant experience",
        PlanOperationIR::DerivePostconditions => "Derive completion conditions",
        PlanOperationIR::ModelKnowledgeGap => "Model the knowledge gap",
        PlanOperationIR::GenerateCandidates => "Generate candidates",
        PlanOperationIR::PredictConsequences => "Predict consequences",
        PlanOperationIR::RunDiagnostic => "Run a diagnostic",
        PlanOperationIR::ValidateCandidates => "Validate candidates",
        PlanOperationIR::ApplySelectedAction => "Apply the selected action",
        PlanOperationIR::VerifyOutcome => "Verify the outcome",
        PlanOperationIR::GeneralizeLesson => "Generalize the lesson",
        PlanOperationIR::StoreSuccessfulExperience => "Store successful experience",
        PlanOperationIR::SynthesizeExplanation => "Synthesize an explanation",
        PlanOperationIR::CommunicateResult => "Communicate the result",
    }
}

#[cfg(test)]
mod tests {
    use dockable_semantic_core::{ExperienceOutcomeIR, EXPERIENCE_SCHEMA};

    use super::*;

    fn experience() -> ExperienceIR {
        ExperienceIR {
            schema: EXPERIENCE_SCHEMA.to_string(),
            experience_id: "EXP-POWERSHELL-PATH-1".to_string(),
            situation: "PowerShell path handling failed during a Rust build".to_string(),
            action: "use LiteralPath and preserve the exact predecessor".to_string(),
            outcome: ExperienceOutcomeIR::Successful,
            outcome_description: "the build completed and the target path remained exact"
                .to_string(),
            semantic_tags: vec![
                "repair".to_string(),
                "powershell".to_string(),
                "path".to_string(),
            ],
            evidence: vec!["exit_code=0".to_string()],
            confidence_millis: 970,
            source_language: Some("en".to_string()),
        }
    }

    #[test]
    fn public_api_injects_experience_and_grounds_korean_plan_and_output() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        assert!(api.inject_experience(experience()).unwrap().inserted);
        let response = api
            .process(&NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "REQ-KO-1".to_string(),
                text: "파워쉘 경로 오류를 점검하고 수리 계획 세워줘. ㄱㄱ".to_string(),
                output_language: Some(LanguageCodeIR::Korean),
                context_tags: vec!["powershell".to_string(), "path".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert_eq!(
            response.understanding.intent,
            dockable_semantic_core::PlanIntentIR::Repair
        );
        assert_eq!(response.plan.recalled_experiences.len(), 1);
        assert!(response.output.text.contains("관련 성공 경험 1건"));
        assert_eq!(response.output.unsupported_freeform_claims, 0);
    }

    #[test]
    fn json_api_supports_english_input_and_output() {
        let api = CognitiveApi::new_embedded().unwrap();
        let request = serde_json::to_string(&NaturalLanguageRequestIR {
            schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
            request_id: "REQ-EN-1".to_string(),
            text: "FYI, please analyze the root cause and plan a repair".to_string(),
            output_language: Some(LanguageCodeIR::English),
            context_tags: vec!["diagnosis".to_string()],
            max_plan_steps: 12,
        })
        .unwrap();
        let response: NaturalLanguageResponseIR =
            serde_json::from_str(&api.process_json(&request).unwrap()).unwrap();
        assert_eq!(response.output.language, LanguageCodeIR::English);
        assert!(response.output.text.contains("Verify the outcome"));
        assert!(response.plan.structurally_validated);
    }

    #[test]
    fn public_snapshot_api_restores_experience_without_semantic_state_mutation() {
        let mut source = CognitiveApi::new_embedded().unwrap();
        source.inject_experience(experience()).unwrap();
        let snapshot = source.export_experience_snapshot_json().unwrap();
        let mut destination = CognitiveApi::new_embedded().unwrap();
        let receipts: Vec<ExperienceInjectionReceiptIR> = serde_json::from_str(
            &destination
                .import_experience_snapshot_json(&snapshot)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(destination.retained_experience_count(), 1);
    }

    #[test]
    fn command_api_keeps_injected_experience_live_for_following_requests() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let injection = api.execute_command(CognitiveApiCommandIR::InjectExperience {
            experience: experience(),
        });
        assert!(injection.ok);
        let response = api.execute_command(CognitiveApiCommandIR::ProcessNaturalLanguage {
            request: NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "REQ-COMMAND-1".to_string(),
                text: "Please plan a path repair".to_string(),
                output_language: Some(LanguageCodeIR::English),
                context_tags: vec!["path".to_string()],
                max_plan_steps: 12,
            },
        });
        let Some(CognitiveApiPayloadIR::NaturalLanguageResponse(response)) = response.payload
        else {
            panic!("typed natural-language response")
        };
        assert_eq!(response.plan.recalled_experiences.len(), 1);
    }

    #[test]
    fn public_api_rejects_unbounded_context_before_planning() {
        let api = CognitiveApi::new_embedded().unwrap();
        let error = api
            .process(&NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "REQ-OVERSIZED".to_string(),
                text: "plan a repair".to_string(),
                output_language: Some(LanguageCodeIR::English),
                context_tags: (0..65).map(|index| format!("tag-{index}")).collect(),
                max_plan_steps: 12,
            })
            .unwrap_err();
        assert_eq!(error, CognitiveApiError::InvalidRequest);
    }
}
