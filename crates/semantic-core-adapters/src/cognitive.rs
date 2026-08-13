use dockable_semantic_core::{
    DockableCore, ExperienceError, ExperienceIR, ExperienceInjectionReceiptIR,
    ExperienceSnapshotIR, PlanGoalIR, PlanIR, PlanOperationIR, PlanningError, PLAN_GOAL_SCHEMA,
};
use serde::{Deserialize, Serialize};

use crate::knowledge_work::{
    execute_document_work_as_with_reasoning, infer_operation, DocumentKindIR, KnowledgeWorkError,
    KnowledgeWorkOperationIR, KnowledgeWorkProductIR, KnowledgeWorkRequestIR,
    KNOWLEDGE_WORK_RESPONSE_SCHEMA,
};
use crate::language_knowledge::{
    LanguageCodeIR, LanguageKnowledgeBase, LanguageKnowledgeEntryIR, LanguageKnowledgeError,
    LanguageKnowledgeStatisticsIR, LanguageUnderstandingIR,
};
use crate::lexical_memory::{
    ActivatedSenseIR, LexemeIR, LexemeSnapshotIR, LexicalMemory, LexicalMemoryError,
    LexicalMemoryStatisticsIR, LexicalOutcomeIR,
};
use crate::long_term_repair::{
    process_long_term_repair_plan, LongTermRepairPlanError, LongTermRepairPlanRequestIR,
    LongTermRepairPlanResponseIR,
};
use crate::professional_document::{
    process_professional_document, ProfessionalDocumentError, ProfessionalDocumentRequestIR,
    ProfessionalDocumentResponseIR,
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
    pub lexical_activations: Vec<ActivatedSenseIR>,
    pub plan: PlanIR,
    pub output: NaturalLanguageOutputIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeWorkResponseIR {
    pub schema: String,
    pub request_id: String,
    pub understanding: LanguageUnderstandingIR,
    pub lexical_activations: Vec<ActivatedSenseIR>,
    pub plan: PlanIR,
    pub product: KnowledgeWorkProductIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CognitiveApiCommandIR {
    InjectExperience {
        experience: ExperienceIR,
    },
    ExportExperienceSnapshot,
    ImportExperienceSnapshot {
        snapshot: ExperienceSnapshotIR,
    },
    InjectLanguageKnowledge {
        entry: LanguageKnowledgeEntryIR,
    },
    InjectLexeme {
        lexeme: LexemeIR,
    },
    ExportLexemeSnapshot,
    ImportLexemeSnapshot {
        snapshot: LexemeSnapshotIR,
    },
    RecordLexicalOutcome {
        outcome: LexicalOutcomeIR,
    },
    ProcessNaturalLanguage {
        request: NaturalLanguageRequestIR,
    },
    ProcessKnowledgeWork {
        request: KnowledgeWorkRequestIR,
    },
    ProcessLongTermRepairPlan {
        request: LongTermRepairPlanRequestIR,
    },
    ProcessProfessionalDocument {
        request: ProfessionalDocumentRequestIR,
    },
    LanguageKnowledgeStatistics,
    LexicalMemoryStatistics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CognitiveApiPayloadIR {
    ExperienceInjectionReceipt(ExperienceInjectionReceiptIR),
    ExperienceInjectionReceipts(Vec<ExperienceInjectionReceiptIR>),
    ExperienceSnapshot(ExperienceSnapshotIR),
    LanguageKnowledgeInserted(bool),
    LexemeInserted(bool),
    LexemeSnapshot(LexemeSnapshotIR),
    LexemeSnapshotImported,
    LexicalOutcomeRecorded,
    NaturalLanguageResponse(Box<NaturalLanguageResponseIR>),
    KnowledgeWorkResponse(Box<KnowledgeWorkResponseIR>),
    LongTermRepairPlanResponse(Box<LongTermRepairPlanResponseIR>),
    ProfessionalDocumentResponse(Box<ProfessionalDocumentResponseIR>),
    LanguageKnowledgeStatistics(LanguageKnowledgeStatisticsIR),
    LexicalMemoryStatistics(LexicalMemoryStatisticsIR),
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
    LexicalMemory,
    KnowledgeWork,
    LongTermRepairPlan,
    ProfessionalDocument,
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
    lexical_memory: LexicalMemory,
}

impl CognitiveApi {
    pub fn new_embedded() -> Result<Self, CognitiveApiError> {
        Ok(Self {
            core: DockableCore::load_embedded().map_err(|_| CognitiveApiError::CoreLoad)?,
            language_knowledge: LanguageKnowledgeBase::default(),
            lexical_memory: LexicalMemory::default(),
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
        &mut self,
        request: &NaturalLanguageRequestIR,
    ) -> Result<NaturalLanguageResponseIR, CognitiveApiError> {
        validate_request(request)?;
        let mut understanding = self
            .language_knowledge
            .understand(&request.text)
            .map_err(map_language_error)?;
        let lexical_activations = self
            .lexical_memory
            .activate(&request.text, &request.context_tags);
        merge_lexical_activations(&mut understanding, &lexical_activations);
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
            lexical_activations,
            plan,
            output,
        })
    }

    pub fn process_json(&mut self, json: &str) -> Result<String, CognitiveApiError> {
        let request = serde_json::from_str::<NaturalLanguageRequestIR>(json)
            .map_err(|_| CognitiveApiError::JsonInput)?;
        serde_json::to_string(&self.process(&request)?).map_err(|_| CognitiveApiError::JsonOutput)
    }

    pub fn language_knowledge_statistics(&self) -> LanguageKnowledgeStatisticsIR {
        self.language_knowledge.statistics()
    }

    pub fn inject_lexeme(&mut self, lexeme: LexemeIR) -> Result<bool, CognitiveApiError> {
        self.lexical_memory
            .inject(lexeme)
            .map_err(map_lexical_error)
    }

    pub fn export_lexeme_snapshot(&self) -> LexemeSnapshotIR {
        self.lexical_memory.snapshot()
    }

    pub fn import_lexeme_snapshot(
        &mut self,
        snapshot: &LexemeSnapshotIR,
    ) -> Result<(), CognitiveApiError> {
        self.lexical_memory
            .import_snapshot(snapshot)
            .map_err(map_lexical_error)
    }

    pub fn record_lexical_outcome(
        &mut self,
        outcome: &LexicalOutcomeIR,
    ) -> Result<(), CognitiveApiError> {
        self.lexical_memory
            .record_outcome(outcome)
            .map_err(map_lexical_error)
    }

    pub fn lexical_memory_statistics(&self) -> LexicalMemoryStatisticsIR {
        self.lexical_memory.statistics()
    }

    pub fn process_knowledge_work(
        &mut self,
        request: &KnowledgeWorkRequestIR,
    ) -> Result<KnowledgeWorkResponseIR, CognitiveApiError> {
        crate::knowledge_work::validate_request(request).map_err(map_knowledge_work_error)?;
        let mut understanding = self
            .language_knowledge
            .understand(&request.command)
            .map_err(map_language_error)?;
        let lexical_activations = self
            .lexical_memory
            .activate(&request.command, &request.context_tags);
        merge_lexical_activations(&mut understanding, &lexical_activations);
        let operation =
            lexical_knowledge_operation(infer_operation(&request.command), &lexical_activations);
        let document_kind = lexical_document_kind(&lexical_activations);
        understanding.intent = intent_for_knowledge_operation(operation);
        understanding
            .semantic_tags
            .extend(request.context_tags.iter().cloned());
        understanding
            .semantic_tags
            .push("knowledge_work".to_string());
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
                desired_outcomes: vec![
                    "the requested document operation produces a structurally validated artifact"
                        .to_string(),
                    "every analytical finding remains bound to an observable source location"
                        .to_string(),
                    "only the expert roles required by observed quality criteria are spawned"
                        .to_string(),
                    "rendering occurs only after independent assessment and peer review"
                        .to_string(),
                ],
                context_tags: understanding.semantic_tags.clone(),
                max_steps: request.max_plan_steps,
            })
            .map_err(map_planning_error)?;
        let product = execute_document_work_as_with_reasoning(
            request,
            operation,
            document_kind,
            Some(&self.core),
            Some(&plan.plan_sha256),
        )
        .map_err(map_knowledge_work_error)?;
        Ok(KnowledgeWorkResponseIR {
            schema: KNOWLEDGE_WORK_RESPONSE_SCHEMA.to_string(),
            request_id: request.request_id.clone(),
            understanding,
            lexical_activations,
            plan,
            product,
        })
    }

    pub fn process_long_term_repair_plan(
        &mut self,
        request: &LongTermRepairPlanRequestIR,
    ) -> Result<LongTermRepairPlanResponseIR, CognitiveApiError> {
        let mut understanding = self
            .language_knowledge
            .understand(&request.command)
            .map_err(map_language_error)?;
        let lexical_activations = self
            .lexical_memory
            .activate(&request.command, &["long_term_repair_plan".to_string()]);
        merge_lexical_activations(&mut understanding, &lexical_activations);
        understanding.semantic_tags.extend([
            "long_term_repair_plan".to_string(),
            "evidence_bound_document".to_string(),
        ]);
        understanding.semantic_tags.sort();
        understanding.semantic_tags.dedup();
        let plan = self
            .core
            .generate_plan(&PlanGoalIR {
                schema: PLAN_GOAL_SCHEMA.to_string(),
                goal_id: request.request_id.clone(),
                intent: dockable_semantic_core::PlanIntentIR::Plan,
                subject: if understanding.subject.trim().is_empty() {
                    "대한민국 공동주택 장기수선계획".to_string()
                } else {
                    understanding.subject
                },
                constraints: vec![
                    "모든 입력파일은 추출 영수증과 해시로 근거에 결합한다".to_string(),
                    "69개 공사종별과 7개 시설군을 빠짐없이 대사한다".to_string(),
                    "금액과 40년 일정은 고정소수점 계산 엔진 결과만 사용한다".to_string(),
                    "누락값은 0이 아니라 확인 필요로 유지한다".to_string(),
                    "법령·공식안내·단지규약 충돌은 자동 은폐하지 않는다".to_string(),
                    "내부 전문가 검토 외 외부 모델을 호출하지 않는다".to_string(),
                ],
                desired_outcomes: vec![
                    "정확히 50개의 A4 페이지 IR과 인쇄 가능한 HTML을 만든다".to_string(),
                    "시설·비용·충당금·집행 증빙을 동일 항목 ID로 연결한다".to_string(),
                    "전문가가 확인해야 할 입력과 법적 판단을 명확히 분리한다".to_string(),
                ],
                context_tags: understanding.semantic_tags,
                max_steps: request.max_plan_steps,
            })
            .map_err(map_planning_error)?;
        process_long_term_repair_plan(&self.core, request, &plan.plan_sha256)
            .map_err(map_long_term_repair_error)
    }

    pub fn process_professional_document(
        &mut self,
        request: &ProfessionalDocumentRequestIR,
    ) -> Result<ProfessionalDocumentResponseIR, CognitiveApiError> {
        let mut understanding = self
            .language_knowledge
            .understand(&request.command)
            .map_err(map_language_error)?;
        let lexical_activations = self.lexical_memory.activate(
            &request.command,
            &[
                "professional_document".to_string(),
                "long_form_writing".to_string(),
            ],
        );
        merge_lexical_activations(&mut understanding, &lexical_activations);
        understanding.semantic_tags.extend([
            "professional_document".to_string(),
            "evidence_bound_section_synthesis".to_string(),
            "working_memory".to_string(),
            "global_consistency_revision".to_string(),
        ]);
        understanding.semantic_tags.sort();
        understanding.semantic_tags.dedup();
        let plan = self
            .core
            .generate_plan(&PlanGoalIR {
                schema: PLAN_GOAL_SCHEMA.to_string(),
                goal_id: request.request_id.clone(),
                intent: dockable_semantic_core::PlanIntentIR::Create,
                subject: request.title.clone(),
                constraints: vec![
                    format!("exact A4 page budget: {}", request.target_page_count),
                    "every factual paragraph retains an evidence and source-location binding"
                        .to_string(),
                    "missing evidence remains explicit and is never rendered as zero".to_string(),
                    "working memory preserves canonical terms, numeric facts, and open issues"
                        .to_string(),
                    "global consistency is rechecked after every bounded revision".to_string(),
                ],
                desired_outcomes: vec![
                    "produce a dependency-ordered long-form document plan".to_string(),
                    "synthesize each section only from observable evidence".to_string(),
                    "iterate drafting and correction without external model calls".to_string(),
                ],
                context_tags: understanding.semantic_tags,
                max_steps: request.max_plan_steps,
            })
            .map_err(map_planning_error)?;
        process_professional_document(&self.core, request, &plan.plan_sha256)
            .map_err(map_professional_document_error)
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
            CognitiveApiCommandIR::InjectLexeme { lexeme } => self
                .inject_lexeme(lexeme)
                .map(CognitiveApiPayloadIR::LexemeInserted),
            CognitiveApiCommandIR::ExportLexemeSnapshot => Ok(
                CognitiveApiPayloadIR::LexemeSnapshot(self.export_lexeme_snapshot()),
            ),
            CognitiveApiCommandIR::ImportLexemeSnapshot { snapshot } => self
                .import_lexeme_snapshot(&snapshot)
                .map(|()| CognitiveApiPayloadIR::LexemeSnapshotImported),
            CognitiveApiCommandIR::RecordLexicalOutcome { outcome } => self
                .record_lexical_outcome(&outcome)
                .map(|()| CognitiveApiPayloadIR::LexicalOutcomeRecorded),
            CognitiveApiCommandIR::ProcessNaturalLanguage { request } => self
                .process(&request)
                .map(|response| CognitiveApiPayloadIR::NaturalLanguageResponse(Box::new(response))),
            CognitiveApiCommandIR::ProcessKnowledgeWork { request } => self
                .process_knowledge_work(&request)
                .map(|response| CognitiveApiPayloadIR::KnowledgeWorkResponse(Box::new(response))),
            CognitiveApiCommandIR::ProcessLongTermRepairPlan { request } => self
                .process_long_term_repair_plan(&request)
                .map(|response| {
                    CognitiveApiPayloadIR::LongTermRepairPlanResponse(Box::new(response))
                }),
            CognitiveApiCommandIR::ProcessProfessionalDocument { request } => self
                .process_professional_document(&request)
                .map(|response| {
                    CognitiveApiPayloadIR::ProfessionalDocumentResponse(Box::new(response))
                }),
            CognitiveApiCommandIR::LanguageKnowledgeStatistics => {
                Ok(CognitiveApiPayloadIR::LanguageKnowledgeStatistics(
                    self.language_knowledge_statistics(),
                ))
            }
            CognitiveApiCommandIR::LexicalMemoryStatistics => Ok(
                CognitiveApiPayloadIR::LexicalMemoryStatistics(self.lexical_memory_statistics()),
            ),
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

fn map_lexical_error(_: LexicalMemoryError) -> CognitiveApiError {
    CognitiveApiError::LexicalMemory
}

fn map_knowledge_work_error(_: KnowledgeWorkError) -> CognitiveApiError {
    CognitiveApiError::KnowledgeWork
}

fn map_long_term_repair_error(_: LongTermRepairPlanError) -> CognitiveApiError {
    CognitiveApiError::LongTermRepairPlan
}

fn map_professional_document_error(_: ProfessionalDocumentError) -> CognitiveApiError {
    CognitiveApiError::ProfessionalDocument
}

fn map_planning_error(_: PlanningError) -> CognitiveApiError {
    CognitiveApiError::Planning
}

fn merge_lexical_activations(
    understanding: &mut LanguageUnderstandingIR,
    activations: &[ActivatedSenseIR],
) {
    let had_legacy_match = !understanding.matched_knowledge_ids.is_empty();
    let mut observed_lexemes = std::collections::BTreeSet::new();
    let mut strongest_intent = None::<(dockable_semantic_core::PlanIntentIR, u32)>;
    for activation in activations {
        if !observed_lexemes.insert(activation.lexeme_id.as_str()) {
            continue;
        }
        understanding
            .matched_knowledge_ids
            .push(format!("{}/{}", activation.lexeme_id, activation.sense_id));
        understanding
            .semantic_tags
            .push(activation.canonical_concept.clone());
        understanding
            .semantic_tags
            .extend(activation.semantic_tags.iter().cloned());
        if let Some(intent) = activation.intent_hint {
            if strongest_intent
                .as_ref()
                .is_none_or(|(_, score)| activation.activation_millis > *score)
            {
                strongest_intent = Some((intent, activation.activation_millis));
            }
        }
    }
    if !had_legacy_match {
        if let Some((intent, _)) = strongest_intent {
            understanding.intent = intent;
        }
    }
    understanding.matched_knowledge_ids.sort();
    understanding.matched_knowledge_ids.dedup();
    understanding.semantic_tags.sort();
    understanding.semantic_tags.dedup();
}

fn intent_for_knowledge_operation(
    operation: KnowledgeWorkOperationIR,
) -> dockable_semantic_core::PlanIntentIR {
    match operation {
        KnowledgeWorkOperationIR::Interpret | KnowledgeWorkOperationIR::Analyze => {
            dockable_semantic_core::PlanIntentIR::Investigate
        }
        KnowledgeWorkOperationIR::Write | KnowledgeWorkOperationIR::Revise => {
            dockable_semantic_core::PlanIntentIR::Create
        }
        KnowledgeWorkOperationIR::Plan => dockable_semantic_core::PlanIntentIR::Plan,
    }
}

fn lexical_knowledge_operation(
    fallback: KnowledgeWorkOperationIR,
    activations: &[ActivatedSenseIR],
) -> KnowledgeWorkOperationIR {
    activations
        .iter()
        .filter_map(|activation| {
            let operation = match activation.canonical_concept.as_str() {
                "revise" => KnowledgeWorkOperationIR::Revise,
                "author" => KnowledgeWorkOperationIR::Write,
                "plan" => KnowledgeWorkOperationIR::Plan,
                "analyze" => KnowledgeWorkOperationIR::Analyze,
                _ => return None,
            };
            Some((operation, activation.activation_millis))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(operation, _)| operation)
        .unwrap_or(fallback)
}

fn lexical_document_kind(activations: &[ActivatedSenseIR]) -> Option<DocumentKindIR> {
    activations
        .iter()
        .filter_map(|activation| {
            let kind = match activation.canonical_concept.as_str() {
                "academic_paper" => DocumentKindIR::Paper,
                "business_plan" => DocumentKindIR::BusinessPlan,
                "business_proposal" => DocumentKindIR::BusinessProposal,
                "user_guide" => DocumentKindIR::UserGuide,
                "data_table" => DocumentKindIR::Table,
                "data_chart" => DocumentKindIR::Chart,
                "financial_statement" => DocumentKindIR::FinancialStatement,
                _ => return None,
            };
            Some((kind, activation.activation_millis))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(kind, _)| kind)
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
    use crate::knowledge_work::{
        DocumentKindIR, KnowledgeDocumentIR, KnowledgeSourceIR, OutputDirectiveIR, OutputFormatIR,
        OutputModeIR, PlanProposalIR, SourceTextFormatIR, KNOWLEDGE_WORK_REQUEST_SCHEMA,
        PLAN_PROPOSAL_SCHEMA,
    };
    use crate::lexical_memory::{PartOfSpeechIR, SenseIR, LEXEME_SCHEMA};

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
        let mut api = CognitiveApi::new_embedded().unwrap();
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
        let mut api = CognitiveApi::new_embedded().unwrap();
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

    #[test]
    fn natural_language_knowledge_work_closes_lexeme_plan_analysis_and_text_output() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-FINANCE-1".to_string(),
                command: "이 재무제표를 분석하고 회계 등식도 확인해".to_string(),
                source: Some(KnowledgeSourceIR::Text {
                    text: "항목,2025,2026\n총자산,100,120\n총부채,40,50\n총자본,60,70".to_string(),
                    format: Some(SourceTextFormatIR::Csv),
                }),
                document_kind: None,
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Markdown,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["finance".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert_eq!(
            response.product.document.kind(),
            DocumentKindIR::FinancialStatement
        );
        assert!(response
            .lexical_activations
            .iter()
            .any(|activation| activation.canonical_concept == "financial_statement"));
        assert!(response
            .product
            .findings
            .iter()
            .any(|finding| finding.statement.contains("자산 = 부채 + 자본")));
        assert!(response
            .product
            .text_output
            .as_deref()
            .is_some_and(|text| text.contains("분석 결과")));
        assert!(response.plan.structurally_validated);
    }

    #[test]
    fn command_api_persists_verified_sense_weight_separately_from_encounters() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process(&NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "LEX-1".to_string(),
                text: "표를 분석해".to_string(),
                output_language: Some(LanguageCodeIR::Korean),
                context_tags: vec!["data".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        let activation = first
            .lexical_activations
            .iter()
            .find(|activation| activation.lexeme_id == "KO.TABLE")
            .unwrap();
        api.record_lexical_outcome(&LexicalOutcomeIR {
            activation_keys: vec![format!("{}/{}", activation.lexeme_id, activation.sense_id)],
            verified_success: true,
            evidence: vec!["human-confirmed table interpretation".to_string()],
        })
        .unwrap();
        assert_eq!(api.lexical_memory_statistics().verified_successes, 1);
        let snapshot = api.export_lexeme_snapshot();
        assert!(snapshot.entries.iter().any(|entry| {
            entry
                .usage
                .sense_usage
                .values()
                .any(|usage| usage.verified_success_count == 1)
        }));
    }

    #[test]
    fn injected_lexeme_can_drive_a_new_natural_language_revision_command() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.inject_lexeme(LexemeIR {
            schema: LEXEME_SCHEMA.to_string(),
            lexeme_id: "KO.CUSTOM.REFINE".to_string(),
            language: LanguageCodeIR::Korean,
            lemma: "정련해".to_string(),
            inflected_forms: vec!["정련".to_string()],
            part_of_speech: PartOfSpeechIR::Verb,
            grammatical_roles: Vec::new(),
            senses: vec![SenseIR {
                sense_id: "KO.CUSTOM.REFINE.S1".to_string(),
                canonical_concept: "revise".to_string(),
                gloss: "지정된 문서 구조를 다듬다".to_string(),
                semantic_tags: vec!["revision".to_string()],
                context_selectors: Vec::new(),
                relations: Vec::new(),
                intent_hint: Some(dockable_semantic_core::PlanIntentIR::Create),
                confidence_millis: 1_000,
            }],
            collocations: Vec::new(),
            domains: vec!["document".to_string()],
            source: "operator supplied terminology".to_string(),
            confidence_millis: 1_000,
            frequency_prior: 1,
        })
        .unwrap();
        let response = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-CUSTOM-1".to_string(),
                command: "정련해\n제목: 검증 가능한 실행안".to_string(),
                source: Some(KnowledgeSourceIR::Structured {
                    document: Box::new(KnowledgeDocumentIR::PlanProposal(PlanProposalIR {
                        schema: PLAN_PROPOSAL_SCHEMA.to_string(),
                        document_id: "PLAN-1".to_string(),
                        title: "이전 계획".to_string(),
                        objective: "목표".to_string(),
                        tasks: Vec::new(),
                        risks: Vec::new(),
                        assumptions: Vec::new(),
                    })),
                }),
                document_kind: None,
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Markdown,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["document".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert_eq!(response.product.operation, KnowledgeWorkOperationIR::Revise);
        let KnowledgeDocumentIR::PlanProposal(plan) = response.product.document else {
            panic!("plan proposal")
        };
        assert_eq!(plan.title, "검증 가능한 실행안");
    }

    #[test]
    fn chart_command_writes_a_real_svg_file_and_returns_a_receipt() {
        let root =
            std::env::temp_dir().join(format!("b-core-cognitive-chart-{}", std::process::id()));
        let path = root.join("trend.svg");
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-CHART-1".to_string(),
                command: "이 데이터로 선형 차트를 작성해".to_string(),
                source: Some(KnowledgeSourceIR::Text {
                    text: "period,value\nQ1,10\nQ2,15\nQ3,25".to_string(),
                    format: Some(SourceTextFormatIR::Csv),
                }),
                document_kind: Some(DocumentKindIR::Chart),
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::File,
                    format: OutputFormatIR::Svg,
                    path: Some(path.to_string_lossy().to_string()),
                    overwrite: true,
                },
                context_tags: vec!["data".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert!(response.product.text_output.is_none());
        assert!(response.product.file_output.is_some());
        let svg = std::fs::read_to_string(&path).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("polyline"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bilingual_business_genres_drive_the_cognitive_document_pipeline() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let korean = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-BUSINESS-KO".to_string(),
                command: "시장 표와 성장 차트를 포함한 사업계획서를 작성해".to_string(),
                source: None,
                document_kind: None,
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Html,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["business".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert_eq!(korean.product.document.kind(), DocumentKindIR::BusinessPlan);
        assert_eq!(
            korean.product.design.theme,
            crate::knowledge_work::DocumentThemeIR::ExecutiveNavy
        );
        assert!(korean
            .lexical_activations
            .iter()
            .any(|activation| activation.canonical_concept == "business_plan"));
        assert!(korean
            .product
            .text_output
            .as_deref()
            .is_some_and(|html| html.contains("BUSINESS PLAN")));

        let english = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-BUSINESS-EN".to_string(),
                command: "Create a client business proposal with an executive chart".to_string(),
                source: None,
                document_kind: None,
                output_language: Some(LanguageCodeIR::English),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Html,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["proposal".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert_eq!(
            english.product.document.kind(),
            DocumentKindIR::BusinessProposal
        );
        assert!(english
            .lexical_activations
            .iter()
            .any(|activation| activation.canonical_concept == "business_proposal"));
    }

    #[test]
    fn natural_language_manual_activates_guide_without_false_table_activation() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-GUIDE-KO".to_string(),
                command: "GPT 사용 설명서를 작성해. 모르는 기능은 확인 필요라고 표시해."
                    .to_string(),
                source: None,
                document_kind: None,
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Html,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["manual".to_string()],
                max_plan_steps: 16,
            })
            .unwrap();
        assert_eq!(response.product.document.kind(), DocumentKindIR::UserGuide);
        assert_eq!(
            response.product.deliberation.swarm.parent_reasoning_sha256,
            response.plan.plan_sha256
        );
        assert!(response.product.deliberation.causally_gated);
        assert!(response.product.deliberation.render_authorized);
        assert_eq!(response.product.deliberation.swarm.external_model_calls, 0);
        assert!(response
            .lexical_activations
            .iter()
            .any(|activation| activation.canonical_concept == "user_guide"));
        assert!(!response
            .lexical_activations
            .iter()
            .any(|activation| activation.lexeme_id == "KO.TABLE"));
    }

    #[test]
    fn professional_a4_manual_remains_an_authored_guide_and_binds_swarm_to_plan() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let command =
            "GPT 사용 설명서를 전문 A4 문서로 작성해. 확인되지 않은 기능은 확인 필요라고 표시해.";
        let activations = api.lexical_memory.activate(
            command,
            &["manual".to_string(), "professional_document".to_string()],
        );
        assert_eq!(
            lexical_knowledge_operation(infer_operation(command), &activations),
            KnowledgeWorkOperationIR::Write
        );
        let response = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-GUIDE-A4".to_string(),
                command: command.to_string(),
                source: None,
                document_kind: Some(DocumentKindIR::UserGuide),
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Html,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["manual".to_string(), "professional_document".to_string()],
                max_plan_steps: 16,
            })
            .expect("reasoned professional guide");
        assert_eq!(
            response.product.deliberation.swarm.parent_reasoning_sha256,
            response.plan.plan_sha256
        );
    }
}
