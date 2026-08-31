use dockable_semantic_core::{
    DeliberationError, DeliberationIR, DeliberationRequestIR, DeliberationRevisionIR,
    DeliberationRevisionRequestIR, DockableCore, ExperienceError, ExperienceIR,
    ExperienceInjectionReceiptIR, ExperienceSnapshotIR, KnowledgeGroundedDeliberationIR,
    MechanismKnowledgeIR, MechanismKnowledgeInjectionReceiptIR, MechanismMemoryError,
    MechanismMemorySnapshotIR, MechanismQueryIR, PlanGoalIR, PlanIR, PlanOperationIR,
    PlanningError, PLAN_GOAL_SCHEMA,
};
use serde::{Deserialize, Serialize};

use crate::compositional_semantics::{
    PredicateLexemeError, PredicateLexemeIR, PredicateLexiconSnapshotIR,
};
use crate::conditional_guard::ConditionalGuardEvaluationIR;
use crate::conversation::{
    ConversationCommitContext, ConversationFrontendError, ConversationGoalFrameIR,
    ConversationMemory, ConversationStateIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DiscourseReferentKindIR, DynamicDiscourseReferentIR,
    NormalizedUtteranceIR, ReferenceResolutionIR, UtteranceNormalizer,
};
use crate::discourse_qa::{DiscourseAnswerIR, DiscourseQaEngine};
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
use crate::mechanism_induction::{
    MechanismInductionDispositionIR, MechanismInductionEngine, MechanismInductionError,
    MechanismInductionIR, MechanismInductionRequestIR,
};
use crate::pragmatic_memory::{PragmaticMemory, PragmaticMemoryError, PragmaticMemoryStateIR};
use crate::pragmatics::{
    PragmaticContextIR, PragmaticInterpretationIR, PragmaticReasoner, SpeechActIR,
};
use crate::professional_document::{
    process_professional_document, ProfessionalDocumentError, ProfessionalDocumentRequestIR,
    ProfessionalDocumentResponseIR,
};
use crate::raw_mechanism_induction::{
    RawMechanismInductionEngine, RawMechanismInductionError, RawMechanismInductionIR,
    RawMechanismInductionRequestIR,
};
use crate::temporal::{
    TemporalAnswerIR, TemporalQaEngine, TemporalSemanticAnalyzer, TemporalTurnAnalysisIR,
};

pub const NATURAL_LANGUAGE_REQUEST_SCHEMA: &str = "B_CORE_NATURAL_LANGUAGE_REQUEST_1";
pub const NATURAL_LANGUAGE_RESPONSE_SCHEMA: &str = "B_CORE_NATURAL_LANGUAGE_RESPONSE_1";
pub const CONVERSATION_TURN_RESPONSE_SCHEMA: &str = "B_CORE_CONVERSATION_TURN_RESPONSE_4";

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
    pub pragmatic_interpretation: PragmaticInterpretationIR,
    pub lexical_activations: Vec<ActivatedSenseIR>,
    pub plan: PlanIR,
    pub output: NaturalLanguageOutputIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationalOutputIR {
    pub language: LanguageCodeIR,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grounded_plan_sha256: Option<String>,
    pub unsupported_freeform_claims: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationTurnResponseIR {
    pub schema: String,
    pub conversation_id: String,
    pub turn_index: u64,
    pub disposition: ConversationTurnDispositionIR,
    pub normalization: NormalizedUtteranceIR,
    pub reference_resolution: ReferenceResolutionIR,
    pub pragmatic_interpretation: PragmaticInterpretationIR,
    pub pragmatic_state: PragmaticMemoryStateIR,
    pub conversation_state: ConversationStateIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grounded_response: Option<Box<NaturalLanguageResponseIR>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discourse_answer: Option<DiscourseAnswerIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal_answer: Option<TemporalAnswerIR>,
    #[serde(default)]
    pub conditional_guard_evaluations: Vec<ConditionalGuardEvaluationIR>,
    pub output: ConversationalOutputIR,
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
pub struct MechanismInductionResponseIR {
    pub induction: MechanismInductionIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub injection_receipt: Option<MechanismKnowledgeInjectionReceiptIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawMechanismInductionResponseIR {
    pub induction: RawMechanismInductionIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub injection_receipt: Option<MechanismKnowledgeInjectionReceiptIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CognitiveApiCommandIR {
    DeliberateProblem {
        request: DeliberationRequestIR,
    },
    ReviseDeliberation {
        request: Box<DeliberationRevisionRequestIR>,
    },
    InjectMechanismKnowledge {
        knowledge: MechanismKnowledgeIR,
    },
    ExportMechanismMemorySnapshot,
    ImportMechanismMemorySnapshot {
        snapshot: MechanismMemorySnapshotIR,
    },
    DeliberateWithKnowledge {
        request: DeliberationRequestIR,
        query: MechanismQueryIR,
    },
    InduceAndInjectMechanismKnowledge {
        request: Box<MechanismInductionRequestIR>,
    },
    InduceAndInjectRawMechanismKnowledge {
        request: Box<RawMechanismInductionRequestIR>,
    },
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
    ProcessConversationTurn {
        request: ConversationTurnRequestIR,
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
    Deliberation(Box<DeliberationIR>),
    DeliberationRevision(Box<DeliberationRevisionIR>),
    KnowledgeGroundedDeliberation(Box<KnowledgeGroundedDeliberationIR>),
    MechanismKnowledgeInjectionReceipt(MechanismKnowledgeInjectionReceiptIR),
    MechanismKnowledgeInjectionReceipts(Vec<MechanismKnowledgeInjectionReceiptIR>),
    MechanismMemorySnapshot(MechanismMemorySnapshotIR),
    MechanismInductionResponse(Box<MechanismInductionResponseIR>),
    RawMechanismInductionResponse(Box<RawMechanismInductionResponseIR>),
    ExperienceInjectionReceipt(ExperienceInjectionReceiptIR),
    ExperienceInjectionReceipts(Vec<ExperienceInjectionReceiptIR>),
    ExperienceSnapshot(ExperienceSnapshotIR),
    LanguageKnowledgeInserted(bool),
    LexemeInserted(bool),
    LexemeSnapshot(LexemeSnapshotIR),
    LexemeSnapshotImported,
    LexicalOutcomeRecorded,
    NaturalLanguageResponse(Box<NaturalLanguageResponseIR>),
    ConversationTurnResponse(Box<ConversationTurnResponseIR>),
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
    ConversationFrontend,
    PragmaticMemory,
    CompositionalPredicate,
    LexicalMemory,
    KnowledgeWork,
    LongTermRepairPlan,
    ProfessionalDocument,
    Deliberation,
    MechanismMemory,
    MechanismInduction,
    RawMechanismInduction,
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
    utterance_normalizer: UtteranceNormalizer,
    pragmatic_reasoner: PragmaticReasoner,
    discourse_qa: DiscourseQaEngine,
    temporal_analyzer: TemporalSemanticAnalyzer,
    temporal_qa: TemporalQaEngine,
    compositional_predicates: Vec<PredicateLexemeIR>,
    pragmatic_memory: PragmaticMemory,
    conversation_memory: ConversationMemory,
    mechanism_induction: MechanismInductionEngine,
    raw_mechanism_induction: RawMechanismInductionEngine,
}

impl CognitiveApi {
    pub fn new_embedded() -> Result<Self, CognitiveApiError> {
        Ok(Self {
            core: DockableCore::load_embedded().map_err(|_| CognitiveApiError::CoreLoad)?,
            language_knowledge: LanguageKnowledgeBase::default(),
            lexical_memory: LexicalMemory::default(),
            utterance_normalizer: UtteranceNormalizer,
            pragmatic_reasoner: PragmaticReasoner,
            discourse_qa: DiscourseQaEngine,
            temporal_analyzer: TemporalSemanticAnalyzer,
            temporal_qa: TemporalQaEngine,
            compositional_predicates: Vec::new(),
            pragmatic_memory: PragmaticMemory::default(),
            conversation_memory: ConversationMemory::default(),
            mechanism_induction: MechanismInductionEngine,
            raw_mechanism_induction: RawMechanismInductionEngine,
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

    /// Runs the core's bounded causal/epistemic deliberation path. The result
    /// is a typed plan and evidence receipt; this API does not execute external
    /// actions or silently broaden the request's authority.
    pub fn deliberate_problem(
        &self,
        request: &DeliberationRequestIR,
    ) -> Result<DeliberationIR, CognitiveApiError> {
        self.core
            .deliberate_problem(request)
            .map_err(map_deliberation_error)
    }

    pub fn revise_deliberation(
        &self,
        request: &DeliberationRevisionRequestIR,
    ) -> Result<DeliberationRevisionIR, CognitiveApiError> {
        self.core
            .revise_deliberation(request)
            .map_err(map_deliberation_error)
    }

    pub fn inject_mechanism_knowledge(
        &mut self,
        knowledge: MechanismKnowledgeIR,
    ) -> Result<MechanismKnowledgeInjectionReceiptIR, CognitiveApiError> {
        self.core
            .inject_mechanism_knowledge(knowledge)
            .map_err(map_mechanism_memory_error)
    }

    pub fn deliberate_with_knowledge(
        &self,
        request: &DeliberationRequestIR,
        query: &MechanismQueryIR,
    ) -> Result<KnowledgeGroundedDeliberationIR, CognitiveApiError> {
        self.core
            .deliberate_with_knowledge(request, query)
            .map_err(map_mechanism_memory_error)
    }

    /// Compiles a language-bound causal claim only when repeated state
    /// transitions and controls support it, then inserts the resulting typed
    /// mechanism into executable memory. Insufficient or contradictory input
    /// returns a non-compiled receipt and performs no memory mutation.
    pub fn induce_and_inject_mechanism_knowledge(
        &mut self,
        request: &MechanismInductionRequestIR,
    ) -> Result<MechanismInductionResponseIR, CognitiveApiError> {
        let induction = self
            .mechanism_induction
            .compile(request)
            .map_err(map_mechanism_induction_error)?;
        let injection_receipt =
            if induction.disposition == MechanismInductionDispositionIR::Compiled {
                Some(
                    self.core
                        .inject_mechanism_knowledge(
                            induction
                                .knowledge
                                .clone()
                                .ok_or(CognitiveApiError::MechanismInduction)?,
                        )
                        .map_err(map_mechanism_memory_error)?,
                )
            } else {
                None
            };
        Ok(MechanismInductionResponseIR {
            induction,
            injection_receipt,
        })
    }

    /// Builds the proposition vocabulary and typed observations from bounded
    /// raw scalar state maps, then uses the same evidence-bound induction and
    /// executable-memory path as the explicit typed API.
    pub fn induce_and_inject_raw_mechanism_knowledge(
        &mut self,
        request: &RawMechanismInductionRequestIR,
    ) -> Result<RawMechanismInductionResponseIR, CognitiveApiError> {
        let induction = self
            .raw_mechanism_induction
            .compile(request)
            .map_err(map_raw_mechanism_induction_error)?;
        let injection_receipt =
            if induction.induction.disposition == MechanismInductionDispositionIR::Compiled {
                Some(
                    self.core
                        .inject_mechanism_knowledge(
                            induction
                                .induction
                                .knowledge
                                .clone()
                                .ok_or(CognitiveApiError::RawMechanismInduction)?,
                        )
                        .map_err(map_mechanism_memory_error)?,
                )
            } else {
                None
            };
        Ok(RawMechanismInductionResponseIR {
            induction,
            injection_receipt,
        })
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

    pub fn inject_compositional_predicate(
        &mut self,
        predicate: PredicateLexemeIR,
    ) -> Result<bool, CognitiveApiError> {
        predicate.validate().map_err(map_predicate_lexeme_error)?;
        if let Some(existing) = self
            .compositional_predicates
            .iter()
            .find(|existing| existing.predicate_id == predicate.predicate_id)
        {
            return if existing == &predicate {
                Ok(false)
            } else {
                Err(CognitiveApiError::CompositionalPredicate)
            };
        }
        self.compositional_predicates.push(predicate);
        self.compositional_predicates
            .sort_by(|left, right| left.predicate_id.cmp(&right.predicate_id));
        Ok(true)
    }

    pub fn export_compositional_predicates(&self) -> PredicateLexiconSnapshotIR {
        PredicateLexiconSnapshotIR::build(self.compositional_predicates.clone())
            .expect("validated in-memory predicate lexicon must serialize")
    }

    pub fn import_compositional_predicates(
        &mut self,
        snapshot: &PredicateLexiconSnapshotIR,
    ) -> Result<(), CognitiveApiError> {
        snapshot.validate().map_err(map_predicate_lexeme_error)?;
        self.compositional_predicates = snapshot.entries.clone();
        Ok(())
    }

    pub fn process(
        &mut self,
        request: &NaturalLanguageRequestIR,
    ) -> Result<NaturalLanguageResponseIR, CognitiveApiError> {
        let pragmatic_interpretation = self.pragmatic_reasoner.interpret_with_predicates(
            &request.text,
            &PragmaticContextIR::default(),
            &self.compositional_predicates,
        );
        self.process_with_pragmatics(request, pragmatic_interpretation)
    }

    fn process_with_pragmatics(
        &mut self,
        request: &NaturalLanguageRequestIR,
        pragmatic_interpretation: PragmaticInterpretationIR,
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
        pragmatic_interpretation.apply_to_understanding(&mut understanding);
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
            pragmatic_interpretation,
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

    /// Process one turn through the conversational surface layer before the
    /// existing language-independent planner. Empty fillers/backchannels never
    /// create a fake goal, and ambiguous ASR/reference bindings never guess.
    pub fn process_conversation_turn(
        &mut self,
        request: &ConversationTurnRequestIR,
    ) -> Result<ConversationTurnResponseIR, CognitiveApiError> {
        self.conversation_memory
            .validate_turn_order(request)
            .map_err(map_conversation_error)?;
        self.pragmatic_memory
            .validate_turn_order(request)
            .map_err(map_pragmatic_memory_error)?;
        let normalization = self
            .utterance_normalizer
            .normalize(request)
            .map_err(map_conversation_error)?;
        let reference_resolution = self.conversation_memory.resolve_references(
            &request.conversation_id,
            &normalization.semantic_surface_text,
        );
        let mut pragmatic_context = self.pragmatic_memory.context(&request.conversation_id);
        if pragmatic_context.active_subject.is_none() {
            pragmatic_context.active_subject = self
                .conversation_memory
                .state(&request.conversation_id)
                .and_then(|state| state.active_subject.clone());
        }
        let pragmatic_interpretation = self.pragmatic_reasoner.interpret_with_predicates(
            &reference_resolution.resolved_semantic_text,
            &pragmatic_context,
            &self.compositional_predicates,
        );
        let mut disposition = if normalization.disposition
            == ConversationTurnDispositionIR::ClarificationRequired
            || !reference_resolution.ambiguous_reference_surfaces.is_empty()
            || pragmatic_interpretation
                .nonliteral_analysis
                .clarification_required
            || pragmatic_interpretation
                .compositional_analysis
                .clarification_required
        {
            ConversationTurnDispositionIR::ClarificationRequired
        } else {
            normalization.disposition
        };
        let remembered_language = self
            .conversation_memory
            .state(&request.conversation_id)
            .and_then(|state| state.preferred_language);
        let output_language = request
            .output_language
            .filter(|language| matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::English))
            .or(remembered_language)
            .unwrap_or_else(|| conversational_language(&request.raw_text));
        let query_function_reference_only =
            !reference_resolution.ambiguous_reference_surfaces.is_empty()
                && reference_resolution
                    .ambiguous_reference_surfaces
                    .iter()
                    .all(|surface| matches!(surface.to_lowercase().as_str(), "it" | "that"));
        let temporal_answer = if normalization.disposition
            == ConversationTurnDispositionIR::Grounded
            && !normalization.ambiguous_input
            && (reference_resolution.ambiguous_reference_surfaces.is_empty()
                || query_function_reference_only)
        {
            let state = self.conversation_memory.state(&request.conversation_id);
            if query_function_reference_only {
                self.temporal_qa.answer(
                    &normalization.semantic_surface_text,
                    state.map(|state| &state.temporal_graph),
                    output_language,
                )
            } else {
                self.temporal_qa
                    .answer(
                        &reference_resolution.resolved_semantic_text,
                        state.map(|state| &state.temporal_graph),
                        output_language,
                    )
                    .or_else(|| {
                        (reference_resolution.resolved_semantic_text
                            != normalization.semantic_surface_text)
                            .then(|| {
                                self.temporal_qa.answer(
                                    &normalization.semantic_surface_text,
                                    state.map(|state| &state.temporal_graph),
                                    output_language,
                                )
                            })
                            .flatten()
                    })
            }
        } else {
            None
        };
        // A "when did" question presupposes occurrence.  If the only matching
        // event records live in possible/predicted/counterfactual worlds, let
        // the presupposition-aware discourse path abstain instead of answering
        // from a non-actual world as though the event occurred.
        let temporal_answer = temporal_answer.filter(|answer| {
            !(answer.query.kind == crate::temporal::TemporalQueryKindIR::EventTime
                && (answer.disposition
                    == crate::temporal::TemporalAnswerDispositionIR::EventTimeNotRecorded
                    || answer.disposition
                        == crate::temporal::TemporalAnswerDispositionIR::NoMatchingEvent
                    || (!answer.event_evidence.is_empty()
                        && answer.event_evidence.iter().all(|event| {
                            event.modal_world != crate::modality::ModalWorldIR::Actual
                        }))))
        });
        let discourse_answer = if temporal_answer.is_none()
            && normalization.disposition == ConversationTurnDispositionIR::Grounded
            && !normalization.ambiguous_input
            && (reference_resolution.ambiguous_reference_surfaces.is_empty()
                || query_function_reference_only)
        {
            let state = self.conversation_memory.state(&request.conversation_id);
            if query_function_reference_only {
                self.discourse_qa.answer(
                    &normalization.semantic_surface_text,
                    state,
                    output_language,
                )
            } else {
                self.discourse_qa
                    .answer(
                        &reference_resolution.resolved_semantic_text,
                        state,
                        output_language,
                    )
                    .or_else(|| {
                        (reference_resolution.resolved_semantic_text
                            != normalization.semantic_surface_text)
                            .then(|| {
                                self.discourse_qa.answer(
                                    &normalization.semantic_surface_text,
                                    state,
                                    output_language,
                                )
                            })
                            .flatten()
                    })
            }
        } else {
            None
        };
        let has_qa_answer = temporal_answer.is_some() || discourse_answer.is_some();
        let candidate_temporal_analysis = if !has_qa_answer
            && normalization.disposition == ConversationTurnDispositionIR::Grounded
            && !normalization.ambiguous_input
        {
            let temporal_surface = if reference_resolution.ambiguous_reference_surfaces.is_empty() {
                &reference_resolution.resolved_semantic_text
            } else {
                &normalization.semantic_surface_text
            };
            self.temporal_analyzer.analyze_turn(
                temporal_surface,
                request.turn_index,
                self.conversation_memory
                    .state(&request.conversation_id)
                    .map(|state| &state.temporal_graph),
            )
        } else {
            TemporalTurnAnalysisIR::default()
        };
        let temporal_deictic_reference_resolved =
            !candidate_temporal_analysis.relations.is_empty() && query_function_reference_only;
        if has_qa_answer || temporal_deictic_reference_resolved {
            disposition = ConversationTurnDispositionIR::Grounded;
        }

        let (mut grounded_response, mut output, semantic_subject) =
            if disposition == ConversationTurnDispositionIR::Grounded {
                if let Some(answer) = &temporal_answer {
                    (
                        None,
                        ConversationalOutputIR {
                            language: output_language,
                            text: answer.realized_text.clone(),
                            grounded_plan_sha256: None,
                            unsupported_freeform_claims: answer.unsupported_claims,
                        },
                        None,
                    )
                } else if let Some(answer) = &discourse_answer {
                    (
                        None,
                        ConversationalOutputIR {
                            language: output_language,
                            text: answer.realized_text.clone(),
                            grounded_plan_sha256: None,
                            unsupported_freeform_claims: answer.unsupported_claims,
                        },
                        None,
                    )
                } else {
                    let mut context_tags = request.context_tags.clone();
                    context_tags.extend(normalization.semantic_tags.iter().cloned());
                    if let Some(state) = self.conversation_memory.state(&request.conversation_id) {
                        for referent_id in &reference_resolution.used_referent_ids {
                            if let Some(referent) = state
                                .active_referents
                                .iter()
                                .find(|referent| &referent.referent_id == referent_id)
                            {
                                context_tags.push(referent.canonical_concept.clone());
                            }
                        }
                    }
                    context_tags.sort();
                    context_tags.dedup();
                    let mut response = self.process_with_pragmatics(
                        &NaturalLanguageRequestIR {
                            schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                            request_id: request.request_id.clone(),
                            text: reference_resolution.resolved_semantic_text.clone(),
                            output_language: Some(output_language),
                            context_tags,
                            max_plan_steps: request.max_plan_steps,
                        },
                        pragmatic_interpretation.clone(),
                    )?;
                    response.understanding.original_text = request.raw_text.clone();
                    response.understanding.normalized_text =
                        reference_resolution.resolved_semantic_text.clone();
                    let conversational_text = render_conversation_grounding(
                        output_language,
                        &response.understanding,
                        &response.plan,
                        &response.pragmatic_interpretation,
                    );
                    let output = ConversationalOutputIR {
                        language: output_language,
                        text: conversational_text,
                        grounded_plan_sha256: Some(response.plan.plan_sha256.clone()),
                        unsupported_freeform_claims: 0,
                    };
                    let subject = response.understanding.subject.clone();
                    (Some(Box::new(response)), output, Some(subject))
                }
            } else {
                (
                    None,
                    ConversationalOutputIR {
                        language: output_language,
                        text: render_non_grounded_conversation(
                            output_language,
                            disposition,
                            &normalization,
                            &reference_resolution,
                            &pragmatic_interpretation,
                        ),
                        grounded_plan_sha256: None,
                        unsupported_freeform_claims: 0,
                    },
                    None,
                )
            };
        let grounded_goals = if disposition == ConversationTurnDispositionIR::Grounded
            && !has_qa_answer
            && pragmatic_interpretation
                .compositional_analysis
                .modal_scope_graph
                .conditionals
                .is_empty()
        {
            conversation_goal_frames(
                &pragmatic_interpretation,
                request.turn_index,
                &reference_resolution.resolved_semantic_text,
            )
        } else {
            Vec::new()
        };
        let proposition_referents =
            if disposition == ConversationTurnDispositionIR::Grounded && !has_qa_answer {
                conversation_proposition_referents(&pragmatic_interpretation, request.turn_index)
            } else {
                Vec::new()
            };
        let temporal_analysis = if disposition == ConversationTurnDispositionIR::Grounded {
            candidate_temporal_analysis
        } else {
            TemporalTurnAnalysisIR::default()
        };
        let temporal_analysis_ref = (!temporal_analysis.events.is_empty()
            || !temporal_analysis.relations.is_empty())
        .then_some(&temporal_analysis);
        let conversation_state = self
            .conversation_memory
            .commit_turn_with_discourse(
                request,
                ConversationCommitContext {
                    semantic_subject: semantic_subject.as_deref(),
                    used_referent_ids: &reference_resolution.used_referent_ids,
                    unresolved_reference_count: if (has_qa_answer
                        || temporal_deictic_reference_resolved)
                        && query_function_reference_only
                    {
                        usize::from(normalization.ambiguous_input)
                    } else {
                        reference_resolution.ambiguous_reference_surfaces.len()
                            + usize::from(normalization.ambiguous_input)
                    },
                    language: Some(output_language),
                    grounded_goals: &grounded_goals,
                    proposition_referents: &proposition_referents,
                    temporal_analysis: temporal_analysis_ref,
                    guard_conditionals: (disposition == ConversationTurnDispositionIR::Grounded
                        && !has_qa_answer
                        && (!pragmatic_interpretation
                            .compositional_analysis
                            .modal_scope_graph
                            .conditionals
                            .is_empty()
                            || !proposition_referents.is_empty()))
                    .then_some(
                        pragmatic_interpretation
                            .compositional_analysis
                            .modal_scope_graph
                            .conditionals
                            .as_slice(),
                    ),
                },
            )
            .map_err(map_conversation_error)?;
        let conditional_guard_evaluations = conversation_state
            .last_guard_evaluations
            .iter()
            .filter(|evaluation| evaluation.evaluation_turn == request.turn_index)
            .cloned()
            .collect::<Vec<_>>();
        if !conditional_guard_evaluations.is_empty()
            && pragmatic_interpretation.inferred_goal.is_none()
            && !has_qa_answer
        {
            grounded_response = None;
            output.text = conditional_guard_evaluations
                .iter()
                .map(|evaluation| evaluation.realized_text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            output.grounded_plan_sha256 = None;
            output.unsupported_freeform_claims = conditional_guard_evaluations
                .iter()
                .map(|evaluation| evaluation.unsupported_claims)
                .sum();
        }
        let mut memory_interpretation = pragmatic_interpretation.clone();
        if has_qa_answer {
            memory_interpretation.inferred_current_task = None;
            memory_interpretation.inferred_goal = None;
            memory_interpretation.continuation_gate = None;
        }
        let pragmatic_state = self
            .pragmatic_memory
            .commit_turn(request, &memory_interpretation)
            .map_err(map_pragmatic_memory_error)?;
        Ok(ConversationTurnResponseIR {
            schema: CONVERSATION_TURN_RESPONSE_SCHEMA.to_string(),
            conversation_id: request.conversation_id.clone(),
            turn_index: request.turn_index,
            disposition,
            normalization,
            reference_resolution,
            pragmatic_interpretation,
            pragmatic_state,
            conversation_state,
            grounded_response,
            discourse_answer,
            temporal_answer,
            conditional_guard_evaluations,
            output,
        })
    }

    pub fn conversation_state(&self, conversation_id: &str) -> Option<&ConversationStateIR> {
        self.conversation_memory.state(conversation_id)
    }

    pub fn pragmatic_state(&self, conversation_id: &str) -> Option<&PragmaticMemoryStateIR> {
        self.pragmatic_memory.state(conversation_id)
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
            CognitiveApiCommandIR::DeliberateProblem { request } => self
                .deliberate_problem(&request)
                .map(|result| CognitiveApiPayloadIR::Deliberation(Box::new(result))),
            CognitiveApiCommandIR::ReviseDeliberation { request } => self
                .revise_deliberation(&request)
                .map(|result| CognitiveApiPayloadIR::DeliberationRevision(Box::new(result))),
            CognitiveApiCommandIR::InjectMechanismKnowledge { knowledge } => self
                .inject_mechanism_knowledge(knowledge)
                .map(CognitiveApiPayloadIR::MechanismKnowledgeInjectionReceipt),
            CognitiveApiCommandIR::ExportMechanismMemorySnapshot => {
                Ok(CognitiveApiPayloadIR::MechanismMemorySnapshot(
                    self.core.export_mechanism_memory_snapshot(),
                ))
            }
            CognitiveApiCommandIR::ImportMechanismMemorySnapshot { snapshot } => self
                .core
                .import_mechanism_memory_snapshot(&snapshot)
                .map(CognitiveApiPayloadIR::MechanismKnowledgeInjectionReceipts)
                .map_err(map_mechanism_memory_error),
            CognitiveApiCommandIR::DeliberateWithKnowledge { request, query } => self
                .deliberate_with_knowledge(&request, &query)
                .map(|result| {
                    CognitiveApiPayloadIR::KnowledgeGroundedDeliberation(Box::new(result))
                }),
            CognitiveApiCommandIR::InduceAndInjectMechanismKnowledge { request } => self
                .induce_and_inject_mechanism_knowledge(&request)
                .map(|result| CognitiveApiPayloadIR::MechanismInductionResponse(Box::new(result))),
            CognitiveApiCommandIR::InduceAndInjectRawMechanismKnowledge { request } => self
                .induce_and_inject_raw_mechanism_knowledge(&request)
                .map(|result| {
                    CognitiveApiPayloadIR::RawMechanismInductionResponse(Box::new(result))
                }),
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
            CognitiveApiCommandIR::ProcessConversationTurn { request } => {
                self.process_conversation_turn(&request).map(|response| {
                    CognitiveApiPayloadIR::ConversationTurnResponse(Box::new(response))
                })
            }
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

fn map_conversation_error(_: ConversationFrontendError) -> CognitiveApiError {
    CognitiveApiError::ConversationFrontend
}

fn conversation_goal_frames(
    interpretation: &PragmaticInterpretationIR,
    turn_index: u64,
    source_semantic_text: &str,
) -> Vec<ConversationGoalFrameIR> {
    interpretation
        .compositional_analysis
        .selected_candidates()
        .into_iter()
        .filter_map(|candidate| {
            interpretation
                .compositional_analysis
                .frames
                .iter()
                .find(|frame| frame.frame_id == candidate.source_frame_id)
                .map(|frame| (candidate, frame))
        })
        .enumerate()
        .map(|(index, (candidate, frame))| ConversationGoalFrameIR {
            goal_id: format!("GOAL-{turn_index:06}-{:02}", index + 1),
            intent: candidate.intent,
            canonical_predicate: frame.canonical_predicate.clone(),
            predicate_surface: frame.predicate_surface.clone(),
            subject: candidate.subject.clone(),
            source_semantic_text: source_semantic_text.to_string(),
            introduced_turn: turn_index,
            last_referenced_turn: turn_index,
            external_execution_authorized: candidate.external_execution_authorized,
        })
        .collect()
}

fn conversation_proposition_referents(
    interpretation: &PragmaticInterpretationIR,
    turn_index: u64,
) -> Vec<DynamicDiscourseReferentIR> {
    if interpretation
        .clauses
        .iter()
        .any(|clause| crate::epistemic::is_retraction_surface(&clause.surface_text))
    {
        return Vec::new();
    }
    if interpretation.inferred_goal.is_some()
        || !matches!(
            interpretation.speech_act,
            SpeechActIR::Inform
                | SpeechActIR::NegativeEvaluation
                | SpeechActIR::ConditionalCommitment
        )
    {
        return Vec::new();
    }
    let attribution_graph = &interpretation.compositional_analysis.attribution_graph;
    if !attribution_graph.attributions.is_empty() {
        return attribution_graph
            .attributions
            .iter()
            .take(4)
            .enumerate()
            .filter_map(|(index, edge)| {
                let actor = attribution_graph.actor(&edge.actor_id)?;
                let proposition = attribution_graph.proposition(&edge.proposition_id)?;
                Some(DynamicDiscourseReferentIR {
                    referent_id: format!("DREF-P-{turn_index:06}-{:02}", index + 1),
                    kind: DiscourseReferentKindIR::Proposition,
                    semantic_summary: proposition.surface_text.clone(),
                    attributed_source: Some(actor.surface.clone()),
                    attribution_attitude: Some(edge.attitude),
                    epistemic_status: Some(edge.epistemic_status),
                    proposition_polarity: Some(proposition.polarity),
                    modal_world: Some(attributed_modal_world(
                        edge.attitude,
                        &proposition.surface_text,
                    )),
                    belief_record_id: None,
                    introduced_turn: turn_index,
                    last_referenced_turn: turn_index,
                    external_execution_authorized: false,
                })
            })
            .collect();
    }
    interpretation
        .clauses
        .iter()
        .filter(|clause| !clause.surface_text.trim().is_empty())
        .take(4)
        .enumerate()
        .filter_map(|(index, clause)| {
            let polarity = match clause.polarity {
                crate::pragmatics::PropositionPolarityIR::Positive => {
                    crate::attribution::AttributedPropositionPolarityIR::Positive
                }
                crate::pragmatics::PropositionPolarityIR::Negative => {
                    crate::attribution::AttributedPropositionPolarityIR::Negative
                }
                crate::pragmatics::PropositionPolarityIR::Mixed => return None,
            };
            Some(DynamicDiscourseReferentIR {
                referent_id: format!("DREF-P-{turn_index:06}-{:02}", index + 1),
                kind: DiscourseReferentKindIR::Proposition,
                semantic_summary: clause.surface_text.clone(),
                attributed_source: None,
                attribution_attitude: None,
                epistemic_status: None,
                proposition_polarity: Some(polarity),
                modal_world: Some(
                    crate::modality::ModalSemanticAnalyzer
                        .analyze(&clause.surface_text)
                        .root_world,
                ),
                belief_record_id: None,
                introduced_turn: turn_index,
                last_referenced_turn: turn_index,
                external_execution_authorized: false,
            })
        })
        .collect()
}

fn attributed_modal_world(
    attitude: crate::attribution::AttributionAttitudeIR,
    proposition_surface: &str,
) -> crate::modality::ModalWorldIR {
    let parsed = crate::modality::ModalSemanticAnalyzer
        .analyze(proposition_surface)
        .root_world;
    if parsed != crate::modality::ModalWorldIR::Actual {
        return parsed;
    }
    match attitude {
        crate::attribution::AttributionAttitudeIR::Want => crate::modality::ModalWorldIR::Desired,
        crate::attribution::AttributionAttitudeIR::Expect => {
            crate::modality::ModalWorldIR::Predicted
        }
        _ => parsed,
    }
}

fn map_pragmatic_memory_error(_: PragmaticMemoryError) -> CognitiveApiError {
    CognitiveApiError::PragmaticMemory
}

fn map_predicate_lexeme_error(_: PredicateLexemeError) -> CognitiveApiError {
    CognitiveApiError::CompositionalPredicate
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

fn map_deliberation_error(_: DeliberationError) -> CognitiveApiError {
    CognitiveApiError::Deliberation
}

fn map_mechanism_memory_error(_: MechanismMemoryError) -> CognitiveApiError {
    CognitiveApiError::MechanismMemory
}

fn map_mechanism_induction_error(_: MechanismInductionError) -> CognitiveApiError {
    CognitiveApiError::MechanismInduction
}

fn map_raw_mechanism_induction_error(_: RawMechanismInductionError) -> CognitiveApiError {
    CognitiveApiError::RawMechanismInduction
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

fn conversational_language(text: &str) -> LanguageCodeIR {
    if text
        .chars()
        .any(|character| matches!(character, '\u{ac00}'..='\u{d7a3}' | '\u{3131}'..='\u{318e}'))
    {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    }
}

fn render_conversation_grounding(
    language: LanguageCodeIR,
    understanding: &LanguageUnderstandingIR,
    plan: &PlanIR,
    pragmatic_interpretation: &PragmaticInterpretationIR,
) -> String {
    debug_assert!(plan.structurally_validated);
    if let Some(gate) = &pragmatic_interpretation.continuation_gate {
        return match language {
            LanguageCodeIR::Korean => format!(
                "‘{}’ 작업을 계속할지는 ‘{}’라는 실제 이득이 있는지 먼저 검증해야 한다는 뜻으로 이해했어. 이득이 확인되면 계속하고, 아니면 그 결과를 보고한 뒤 멈출지 물을게. 아직 판단할 증거가 부족해도 추측하지 않고 확인을 요청할게.",
                gate.current_task, gate.required_benefit
            ),
            _ => format!(
                "I understood this as a continuation gate for ‘{}’: first verify the real benefit ‘{}’. Continue if it is supported; otherwise report the result and ask whether to stop. If the evidence remains unresolved, ask instead of guessing.",
                gate.current_task, gate.required_benefit
            ),
        };
    }
    if pragmatic_interpretation.nonliteral_analysis.has_sarcasm() {
        return match language {
            LanguageCodeIR::Korean => "표면적으로는 칭찬처럼 보이지만 앞의 실패·오류 상태와 의미가 충돌하므로, 실제 의도는 긍정 승인이 아니라 부정적 평가나 불만에 가깝다고 이해했어. 이 표현만으로 새 작업 권한을 만들지는 않을게.".to_string(),
            _ => "The surface wording looks like praise, but it conflicts with the stated failure state. I therefore read it as a negative evaluation or complaint, not approval, and I will not derive new action authority from it alone.".to_string(),
        };
    }
    if let Some(expression) = pragmatic_interpretation
        .nonliteral_analysis
        .expressions
        .iter()
        .find(|expression| {
            expression.selected_reading == crate::nonliteral::ReadingSelectionIR::Figurative
        })
    {
        return match language {
            LanguageCodeIR::Korean => format!(
                "‘{}’를 문자 그대로의 행동이 아니라 ‘{}’에 해당하는 비유적 상태로 이해했어. 물리적 의미로 실행하지 않고 실제 막힘이나 문제를 확인할게.",
                expression.surface_text, expression.figurative_concept
            ),
            _ => format!(
                "I understood ‘{}’ figuratively as ‘{}’, not as a literal action. I will inspect the actual blocked or problematic state instead of executing the physical reading.",
                expression.surface_text, expression.figurative_concept
            ),
        };
    }
    if let Some(goal) = pragmatic_interpretation
        .inferred_goal
        .as_ref()
        .filter(|goal| !goal.external_execution_authorized)
    {
        return match (language, goal.commitment, goal.intent) {
            (
                LanguageCodeIR::Korean,
                crate::pragmatics::GoalCommitmentIR::Suggestion,
                _,
            ) => format!(
                "‘{}’라는 개선 제안으로 이해했어. 구현 명령으로 단정하지 않고 기대 효과와 요구사항부터 확인할게.",
                goal.subject
            ),
            (
                LanguageCodeIR::Korean,
                crate::pragmatics::GoalCommitmentIR::ImplicitRequest,
                dockable_semantic_core::PlanIntentIR::Investigate,
            ) => format!(
                "‘{}’의 원인이나 이유를 알고 싶다는 뜻으로 이해했어. 관찰 가능한 증거부터 확인할게.",
                goal.subject
            ),
            (
                LanguageCodeIR::Korean,
                crate::pragmatics::GoalCommitmentIR::ImplicitRequest,
                dockable_semantic_core::PlanIntentIR::Repair,
            ) => format!(
                "‘{}’ 상태를 그대로 둘 수 없으니 수리가 필요하다는 뜻으로 이해했어. 원인과 수정 범위를 확인하되, 이 암묵적 표현만으로 외부 변경 권한을 넓히지는 않을게.",
                goal.subject
            ),
            (_, crate::pragmatics::GoalCommitmentIR::Suggestion, _) => format!(
                "I understood ‘{}’ as an improvement suggestion, not automatic authorization to implement it. I'll first clarify its expected benefit and requirements.",
                goal.subject
            ),
            (_, _, dockable_semantic_core::PlanIntentIR::Investigate) => format!(
                "I understood that you want to know the cause or explanation for ‘{}’. I'll start from observable evidence.",
                goal.subject
            ),
            (_, _, _) => format!(
                "I inferred an indirect request concerning ‘{}’. I'll inspect the cause and scope without treating the wording as broader external-mutation authority.",
                goal.subject
            ),
        };
    }
    let subject = understanding.subject.trim();
    match language {
        LanguageCodeIR::Korean => {
            let formal = understanding.detected_register
                == crate::language_knowledge::LanguageRegisterIR::Formal;
            let action = match understanding.intent {
                dockable_semantic_core::PlanIntentIR::Repair => {
                    "현재 상태를 확인하고 원인을 좁힌 뒤, 수정 결과까지 검증"
                }
                dockable_semantic_core::PlanIntentIR::Investigate => {
                    "확인 가능한 증거부터 살펴보고 가능한 원인을 비교"
                }
                dockable_semantic_core::PlanIntentIR::Create => {
                    "요구사항을 정리하고 구현한 다음 결과를 검증"
                }
                dockable_semantic_core::PlanIntentIR::Execute => {
                    "현재 상태를 확인한 뒤 요청한 작업을 실행하고 결과를 검증"
                }
                dockable_semantic_core::PlanIntentIR::Learn => {
                    "부족한 지식을 확인하고 재사용 가능한 방법으로 검증"
                }
                dockable_semantic_core::PlanIntentIR::Explain
                | dockable_semantic_core::PlanIntentIR::Communicate => {
                    "근거를 확인하고 핵심 관계를 정리해서 설명"
                }
                dockable_semantic_core::PlanIntentIR::Plan => "필요한 단계와 검증 순서를 정리",
            };
            if formal {
                format!("알겠습니다. ‘{subject}’ 요청을 기준으로 {action}하겠습니다.")
            } else {
                format!("알겠어. ‘{subject}’ 요청을 기준으로 {action}할게.")
            }
        }
        _ => {
            let action = match understanding.intent {
                dockable_semantic_core::PlanIntentIR::Repair => {
                    "check the current state, narrow down the cause, and verify the repair"
                }
                dockable_semantic_core::PlanIntentIR::Investigate => {
                    "start from observable evidence and compare the plausible causes"
                }
                dockable_semantic_core::PlanIntentIR::Create => {
                    "bind the requirements, implement the change, and verify the result"
                }
                dockable_semantic_core::PlanIntentIR::Execute => {
                    "check the current state, carry out the request, and verify the result"
                }
                dockable_semantic_core::PlanIntentIR::Learn => {
                    "identify the knowledge gap and validate a reusable method"
                }
                dockable_semantic_core::PlanIntentIR::Explain
                | dockable_semantic_core::PlanIntentIR::Communicate => {
                    "check the evidence and explain the important relationships"
                }
                dockable_semantic_core::PlanIntentIR::Plan => {
                    "lay out the required steps and their verification order"
                }
            };
            format!("Got it. For the request ‘{subject}’, I'll {action}.")
        }
    }
}

fn render_non_grounded_conversation(
    language: LanguageCodeIR,
    disposition: ConversationTurnDispositionIR,
    normalization: &NormalizedUtteranceIR,
    resolution: &ReferenceResolutionIR,
    pragmatic_interpretation: &PragmaticInterpretationIR,
) -> String {
    match (language, disposition) {
        (LanguageCodeIR::Korean, ConversationTurnDispositionIR::HoldFloor) => {
            "응, 천천히 말해. 듣고 있어.".to_string()
        }
        (LanguageCodeIR::Korean, ConversationTurnDispositionIR::BackchannelOnly) => {
            if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Greeting)
            {
                "안녕! 무엇을 도와줄까?".to_string()
            } else if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Gratitude)
            {
                "천만에. 더 필요한 게 있으면 말해줘.".to_string()
            } else if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Farewell)
            {
                "좋아. 필요할 때 다시 불러줘.".to_string()
            } else {
                "응, 이어서 말해줘.".to_string()
            }
        }
        (LanguageCodeIR::Korean, ConversationTurnDispositionIR::ClarificationRequired) => {
            if pragmatic_interpretation
                .compositional_analysis
                .clarification_required
            {
                let competition = pragmatic_interpretation
                    .compositional_analysis
                    .unresolved_competitions
                    .first()
                    .map_or("서로 다른 요청 후보", String::as_str);
                format!(
                    "문장에서 {competition}가 비슷한 강도로 해석돼. 어느 쪽이 실제 요청인지 지정해줘. 인용·가정·금지된 행동은 임의로 실행하지 않을게."
                )
            } else if pragmatic_interpretation
                .nonliteral_analysis
                .clarification_required
            {
                let surface = pragmatic_interpretation
                    .nonliteral_analysis
                    .expressions
                    .iter()
                    .find(|expression| {
                        expression.selected_reading
                            == crate::nonliteral::ReadingSelectionIR::Ambiguous
                    })
                    .map_or("해당 표현", |expression| {
                        expression.surface_text.as_str()
                    });
                format!(
                    "‘{surface}’를 문자 그대로의 상황으로 말한 건지, 비유적인 문제 상황으로 말한 건지 확인해줘. 어느 쪽도 추측해서 실행하지 않을게."
                )
            } else if normalization.ambiguous_input {
                let choices = normalization
                    .candidates
                    .iter()
                    .take(2)
                    .map(|candidate| format!("‘{}’", candidate.normalized_text))
                    .collect::<Vec<_>>()
                    .join(" 또는 ");
                format!("음성 입력이 {choices}로 들릴 수 있어. 어느 쪽인지 한 번만 확인해줘.")
            } else if !resolution.ambiguous_reference_surfaces.is_empty() {
                format!(
                    "{}가 무엇을 가리키는지 하나만 지정해줘.",
                    resolution
                        .ambiguous_reference_surfaces
                        .iter()
                        .map(|surface| format!("‘{surface}’"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                "무엇을 원하는지 조금만 더 말해줘.".to_string()
            }
        }
        (_, ConversationTurnDispositionIR::HoldFloor) => {
            "Take your time—I'm listening.".to_string()
        }
        (_, ConversationTurnDispositionIR::BackchannelOnly) => {
            if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Greeting)
            {
                "Hi! What can I help you with?".to_string()
            } else if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Gratitude)
            {
                "You're welcome. Let me know if you need anything else.".to_string()
            } else if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Farewell)
            {
                "Sounds good. Call me again whenever you need me.".to_string()
            } else {
                "Got it. Go on when you're ready.".to_string()
            }
        }
        (_, ConversationTurnDispositionIR::ClarificationRequired) => {
            if pragmatic_interpretation
                .compositional_analysis
                .clarification_required
            {
                let competition = pragmatic_interpretation
                    .compositional_analysis
                    .unresolved_competitions
                    .first()
                    .map_or("competing request readings", String::as_str);
                format!(
                    "The sentence supports {competition} with similar strength. Which one is the actual request? I will not execute quoted, hypothetical, or prohibited actions by guessing."
                )
            } else if pragmatic_interpretation
                .nonliteral_analysis
                .clarification_required
            {
                let surface = pragmatic_interpretation
                    .nonliteral_analysis
                    .expressions
                    .iter()
                    .find(|expression| {
                        expression.selected_reading
                            == crate::nonliteral::ReadingSelectionIR::Ambiguous
                    })
                    .map_or("that expression", |expression| {
                        expression.surface_text.as_str()
                    });
                format!(
                    "Did you mean ‘{surface}’ literally, or as a figurative description of a problem? I won't execute either reading by guessing."
                )
            } else if normalization.ambiguous_input {
                let choices = normalization
                    .candidates
                    .iter()
                    .take(2)
                    .map(|candidate| format!("‘{}’", candidate.normalized_text))
                    .collect::<Vec<_>>()
                    .join(" or ");
                format!("The voice input could be {choices}. Which one did you mean?")
            } else if !resolution.ambiguous_reference_surfaces.is_empty() {
                format!(
                    "What does {} refer to?",
                    resolution
                        .ambiguous_reference_surfaces
                        .iter()
                        .map(|surface| format!("‘{surface}’"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                "Could you add a little more detail about what you want?".to_string()
            }
        }
        (_, ConversationTurnDispositionIR::Grounded) => String::new(),
    }
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
        PlanOperationIR::SurfaceAssumptions => "숨은 가정 표면화",
        PlanOperationIR::DerivePostconditions => "완료 조건 도출",
        PlanOperationIR::ModelKnowledgeGap => "지식 공백 모델링",
        PlanOperationIR::GenerateCandidates => "후보 생성",
        PlanOperationIR::GenerateCompetingHypotheses => "경쟁 가설 생성",
        PlanOperationIR::ConstructCausalModel => "인과 모델 구성",
        PlanOperationIR::PredictConsequences => "결과 예측",
        PlanOperationIR::SimulateCounterfactuals => "반사실 시뮬레이션",
        PlanOperationIR::SelectInformationGainAction => "정보가치 행동 선택",
        PlanOperationIR::RunDiagnostic => "진단 실행",
        PlanOperationIR::ValidateCandidates => "후보 검증",
        PlanOperationIR::ApplySelectedAction => "선택 행동 적용",
        PlanOperationIR::VerifyOutcome => "결과 검증",
        PlanOperationIR::ReplanFromObservation => "관찰 기반 재계획",
        PlanOperationIR::CalibrateConfidence => "확신도 보정",
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
        PlanOperationIR::SurfaceAssumptions => "Surface hidden assumptions",
        PlanOperationIR::DerivePostconditions => "Derive completion conditions",
        PlanOperationIR::ModelKnowledgeGap => "Model the knowledge gap",
        PlanOperationIR::GenerateCandidates => "Generate candidates",
        PlanOperationIR::GenerateCompetingHypotheses => "Generate competing hypotheses",
        PlanOperationIR::ConstructCausalModel => "Construct a causal model",
        PlanOperationIR::PredictConsequences => "Predict consequences",
        PlanOperationIR::SimulateCounterfactuals => "Simulate counterfactuals",
        PlanOperationIR::SelectInformationGainAction => "Select an information-gain action",
        PlanOperationIR::RunDiagnostic => "Run a diagnostic",
        PlanOperationIR::ValidateCandidates => "Validate candidates",
        PlanOperationIR::ApplySelectedAction => "Apply the selected action",
        PlanOperationIR::VerifyOutcome => "Verify the outcome",
        PlanOperationIR::ReplanFromObservation => "Replan from observations",
        PlanOperationIR::CalibrateConfidence => "Calibrate confidence",
        PlanOperationIR::GeneralizeLesson => "Generalize the lesson",
        PlanOperationIR::StoreSuccessfulExperience => "Store successful experience",
        PlanOperationIR::SynthesizeExplanation => "Synthesize an explanation",
        PlanOperationIR::CommunicateResult => "Communicate the result",
    }
}

#[cfg(test)]
mod tests {
    use dockable_semantic_core::{
        ActionAuthorityIR, AuthorityEnvelopeIR, CausalMechanismIR, DeliberationDispositionIR,
        DeliberationRequestIR, EvidenceIR, ExperienceOutcomeIR, LiteralIR, MechanismKindIR,
        MechanismKnowledgeIR, MechanismQueryIR, DELIBERATION_REQUEST_SCHEMA, EXPERIENCE_SCHEMA,
        MECHANISM_KNOWLEDGE_SCHEMA,
    };

    use super::*;
    use crate::knowledge_work::{
        DocumentKindIR, KnowledgeDocumentIR, KnowledgeSourceIR, OutputDirectiveIR, OutputFormatIR,
        OutputModeIR, PlanProposalIR, SourceTextFormatIR, KNOWLEDGE_WORK_REQUEST_SCHEMA,
        PLAN_PROPOSAL_SCHEMA,
    };
    use crate::lexical_memory::{PartOfSpeechIR, SenseIR, LEXEME_SCHEMA};
    use crate::mechanism_induction::{
        PropositionLexemeIR, StateTransitionObservationIR, TransitionArmIR,
        MECHANISM_INDUCTION_REQUEST_SCHEMA,
    };
    use crate::raw_mechanism_induction::{
        ObservedValueIR, RawMechanismInductionRequestIR, RawStateTransitionObservationIR,
        RAW_MECHANISM_INDUCTION_REQUEST_SCHEMA,
    };

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

    fn induction_request(
        request_id: &str,
        knowledge_id: &str,
        mechanism_id: &str,
        cause_id: &str,
        cause_alias: &str,
        effect_id: &str,
        effect_alias: &str,
    ) -> MechanismInductionRequestIR {
        let state = |effect: bool| {
            vec![
                LiteralIR {
                    proposition_id: cause_id.to_string(),
                    value: true,
                },
                LiteralIR {
                    proposition_id: effect_id.to_string(),
                    value: effect,
                },
            ]
        };
        let observation = |suffix: &str, arm, after_effect| StateTransitionObservationIR {
            observation_id: format!("{request_id}-{suffix}"),
            arm,
            before: state(false),
            after: state(after_effect),
            reliability_millis: 950,
            evidence_refs: vec![format!("public-test:{request_id}:{suffix}")],
        };
        MechanismInductionRequestIR {
            schema: MECHANISM_INDUCTION_REQUEST_SCHEMA.to_string(),
            request_id: request_id.to_string(),
            knowledge_id: knowledge_id.to_string(),
            mechanism_id: mechanism_id.to_string(),
            natural_language_statement: format!("{cause_alias} causes {effect_alias}"),
            kind: MechanismKindIR::Inference,
            authority: ActionAuthorityIR::InternalInference,
            authorized: true,
            reversible: true,
            recovery_reference: None,
            semantic_tags: vec!["induced-chain".to_string()],
            proposition_lexicon: vec![
                PropositionLexemeIR {
                    proposition_id: cause_id.to_string(),
                    aliases: vec![cause_alias.to_string()],
                },
                PropositionLexemeIR {
                    proposition_id: effect_id.to_string(),
                    aliases: vec![effect_alias.to_string()],
                },
            ],
            observations: vec![
                observation("POS-1", TransitionArmIR::AppliedSuccess, true),
                observation("POS-2", TransitionArmIR::AppliedSuccess, true),
                observation("CONTROL", TransitionArmIR::NoActionControl, false),
            ],
            minimum_positive_support: 2,
            minimum_confidence_millis: 700,
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
    fn public_command_api_runs_compositional_deliberation_and_enforces_authority() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let literal = |id: &str| LiteralIR {
            proposition_id: id.to_string(),
            value: true,
        };
        let request = DeliberationRequestIR {
            schema: DELIBERATION_REQUEST_SCHEMA.to_string(),
            request_id: "DELIBERATE-PUBLIC-1".to_string(),
            subject: "localize and repair an observed pipeline failure".to_string(),
            evidence: vec![EvidenceIR {
                evidence_id: "E-FAILURE".to_string(),
                literal: literal("FAILURE_OBSERVED"),
                reliability_millis: 980,
                source_ref: "public-test:failure".to_string(),
            }],
            mechanisms: vec![
                CausalMechanismIR {
                    mechanism_id: "LOCALIZE".to_string(),
                    kind: MechanismKindIR::Inference,
                    prerequisites: vec![literal("FAILURE_OBSERVED")],
                    effects: vec![literal("CAUSE_LOCALIZED")],
                    observes: Vec::new(),
                    authority: ActionAuthorityIR::InternalInference,
                    authorized: true,
                    reversible: true,
                    recovery_reference: None,
                    cost_millis: 10,
                    risk_millis: 0,
                    provenance_refs: vec!["public-test:localizer".to_string()],
                },
                CausalMechanismIR {
                    mechanism_id: "REPAIR".to_string(),
                    kind: MechanismKindIR::Intervention,
                    prerequisites: vec![literal("CAUSE_LOCALIZED")],
                    effects: vec![literal("REPAIRED")],
                    observes: Vec::new(),
                    authority: ActionAuthorityIR::ReversibleMutation,
                    authorized: true,
                    reversible: true,
                    recovery_reference: Some("sealed-predecessor:public-test".to_string()),
                    cost_millis: 50,
                    risk_millis: 10,
                    provenance_refs: vec!["public-test:repair".to_string()],
                },
            ],
            goals: vec![literal("REPAIRED")],
            authority_envelope: AuthorityEnvelopeIR::default(),
            immutable_constraints: Vec::new(),
            max_depth: 4,
            beam_width: 8,
            max_hypotheses: 8,
            max_counterfactuals: 8,
        };
        let response = api.execute_command(CognitiveApiCommandIR::DeliberateProblem { request });
        let Some(CognitiveApiPayloadIR::Deliberation(result)) = response.payload else {
            panic!("typed deliberation response")
        };
        assert_eq!(result.disposition, DeliberationDispositionIR::GoalReachable);
        assert_eq!(
            result.selected_plan.unwrap().mechanism_ids,
            ["LOCALIZE", "REPAIR"]
        );
        assert_eq!(result.external_action_execution_events, 0);
        assert_eq!(result.external_model_calls, 0);
    }

    #[test]
    fn public_api_reuses_stored_executable_knowledge_instead_of_prose() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let literal = |id: &str| LiteralIR {
            proposition_id: id.to_string(),
            value: true,
        };
        for (knowledge_id, mechanism_id, prerequisite, effect) in [
            ("K-LOCALIZE", "LOCALIZE", "FAILURE", "CAUSE"),
            ("K-REPAIR", "REPAIR", "CAUSE", "RESTORED"),
        ] {
            let response = api.execute_command(CognitiveApiCommandIR::InjectMechanismKnowledge {
                knowledge: MechanismKnowledgeIR {
                    schema: MECHANISM_KNOWLEDGE_SCHEMA.to_string(),
                    knowledge_id: knowledge_id.to_string(),
                    mechanism: CausalMechanismIR {
                        mechanism_id: mechanism_id.to_string(),
                        kind: MechanismKindIR::Inference,
                        prerequisites: vec![literal(prerequisite)],
                        effects: vec![literal(effect)],
                        observes: Vec::new(),
                        authority: ActionAuthorityIR::InternalInference,
                        authorized: true,
                        reversible: true,
                        recovery_reference: None,
                        cost_millis: 10,
                        risk_millis: 0,
                        provenance_refs: vec![format!("public-test:{knowledge_id}")],
                    },
                    semantic_tags: vec!["repair".to_string()],
                    validation_evidence_refs: vec![format!("public-test:{knowledge_id}:pass")],
                    confidence_millis: 950,
                },
            });
            assert!(response.ok);
        }
        let response = api.execute_command(CognitiveApiCommandIR::DeliberateWithKnowledge {
            request: DeliberationRequestIR {
                schema: DELIBERATION_REQUEST_SCHEMA.to_string(),
                request_id: "KNOWLEDGE-THINK-1".to_string(),
                subject: "reuse and compose executable repair knowledge".to_string(),
                evidence: vec![EvidenceIR {
                    evidence_id: "E-FAILURE".to_string(),
                    literal: literal("FAILURE"),
                    reliability_millis: 990,
                    source_ref: "public-test:failure".to_string(),
                }],
                mechanisms: Vec::new(),
                goals: vec![literal("RESTORED")],
                authority_envelope: AuthorityEnvelopeIR::default(),
                immutable_constraints: Vec::new(),
                max_depth: 4,
                beam_width: 8,
                max_hypotheses: 8,
                max_counterfactuals: 8,
            },
            query: MechanismQueryIR {
                semantic_tags: vec!["repair".to_string()],
                known_proposition_ids: vec!["FAILURE".to_string(), "CAUSE".to_string()],
                goal_proposition_ids: vec!["RESTORED".to_string()],
                max_results: 8,
            },
        });
        let Some(CognitiveApiPayloadIR::KnowledgeGroundedDeliberation(result)) = response.payload
        else {
            panic!("knowledge-grounded deliberation")
        };
        assert_eq!(result.recalled_mechanisms.len(), 2);
        assert_eq!(
            result.deliberation.disposition,
            DeliberationDispositionIR::GoalReachable
        );
    }

    #[test]
    fn public_api_induces_stores_and_composes_observed_mechanisms() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for request in [
            induction_request(
                "INDUCE-A-B",
                "K-A-B",
                "M-A-B",
                "SEED_FACT",
                "seed-fact",
                "MIDDLE_FACT",
                "middle-fact",
            ),
            induction_request(
                "INDUCE-B-C",
                "K-B-C",
                "M-B-C",
                "MIDDLE_FACT",
                "middle-fact",
                "FINAL_FACT",
                "final-fact",
            ),
        ] {
            let response =
                api.execute_command(CognitiveApiCommandIR::InduceAndInjectMechanismKnowledge {
                    request: Box::new(request),
                });
            let Some(CognitiveApiPayloadIR::MechanismInductionResponse(response)) =
                response.payload
            else {
                panic!("mechanism induction response")
            };
            assert_eq!(
                response.induction.disposition,
                MechanismInductionDispositionIR::Compiled
            );
            assert!(response.injection_receipt.unwrap().inserted);
            assert_eq!(response.induction.text_only_authority_events, 0);
        }

        let literal = |id: &str| LiteralIR {
            proposition_id: id.to_string(),
            value: true,
        };
        let result = api
            .deliberate_with_knowledge(
                &DeliberationRequestIR {
                    schema: DELIBERATION_REQUEST_SCHEMA.to_string(),
                    request_id: "DELIBERATE-INDUCED-CHAIN".to_string(),
                    subject: "compose mechanisms learned from controlled observations".to_string(),
                    evidence: vec![EvidenceIR {
                        evidence_id: "E-SEED".to_string(),
                        literal: literal("SEED_FACT"),
                        reliability_millis: 990,
                        source_ref: "public-test:seed".to_string(),
                    }],
                    mechanisms: Vec::new(),
                    goals: vec![literal("FINAL_FACT")],
                    authority_envelope: AuthorityEnvelopeIR::default(),
                    immutable_constraints: Vec::new(),
                    max_depth: 4,
                    beam_width: 8,
                    max_hypotheses: 8,
                    max_counterfactuals: 8,
                },
                &MechanismQueryIR {
                    semantic_tags: vec!["induced-chain".to_string()],
                    known_proposition_ids: vec!["SEED_FACT".to_string(), "MIDDLE_FACT".to_string()],
                    goal_proposition_ids: vec!["FINAL_FACT".to_string()],
                    max_results: 8,
                },
            )
            .unwrap();
        assert_eq!(result.recalled_mechanisms.len(), 2);
        assert_eq!(
            result.deliberation.disposition,
            DeliberationDispositionIR::GoalReachable
        );
        assert_eq!(
            result.deliberation.selected_plan.unwrap().mechanism_ids,
            ["M-A-B", "M-B-C"]
        );
    }

    #[test]
    fn public_api_learns_and_composes_raw_state_maps_without_a_manual_lexicon() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (request_id, cause, effect) in [
            ("RAW-CHAIN-A-B", "seed", "middle"),
            ("RAW-CHAIN-B-C", "middle", "final"),
        ] {
            let state = |effect_value| {
                [
                    (cause.to_string(), ObservedValueIR::Boolean(true)),
                    (effect.to_string(), ObservedValueIR::Boolean(effect_value)),
                ]
                .into_iter()
                .collect()
            };
            let observation = |suffix: &str, arm, after_effect| RawStateTransitionObservationIR {
                observation_id: format!("{request_id}-{suffix}"),
                arm,
                before: state(false),
                after: state(after_effect),
                reliability_millis: 950,
                evidence_refs: vec![format!("public-raw:{request_id}:{suffix}")],
            };
            let response = api.execute_command(
                CognitiveApiCommandIR::InduceAndInjectRawMechanismKnowledge {
                    request: Box::new(RawMechanismInductionRequestIR {
                        schema: RAW_MECHANISM_INDUCTION_REQUEST_SCHEMA.to_string(),
                        request_id: request_id.to_string(),
                        knowledge_id: format!("K-{request_id}"),
                        mechanism_id: format!("M-{request_id}"),
                        natural_language_statement: format!("{cause} causes {effect}"),
                        kind: MechanismKindIR::Inference,
                        authority: ActionAuthorityIR::InternalInference,
                        authorized: true,
                        reversible: true,
                        recovery_reference: None,
                        semantic_tags: vec!["raw-chain".to_string()],
                        observations: vec![
                            observation("P1", TransitionArmIR::AppliedSuccess, true),
                            observation("P2", TransitionArmIR::AppliedSuccess, true),
                            observation("C", TransitionArmIR::NoActionControl, false),
                        ],
                        minimum_positive_support: 2,
                        minimum_confidence_millis: 700,
                    }),
                },
            );
            let Some(CognitiveApiPayloadIR::RawMechanismInductionResponse(response)) =
                response.payload
            else {
                panic!("raw mechanism induction response")
            };
            assert_eq!(
                response.induction.induction.disposition,
                MechanismInductionDispositionIR::Compiled
            );
            assert!(response.injection_receipt.unwrap().inserted);
            assert_eq!(response.induction.explicit_proposition_lexicon_entries, 0);
        }

        let literal = |id: &str| LiteralIR {
            proposition_id: id.to_string(),
            value: true,
        };
        let result = api
            .deliberate_with_knowledge(
                &DeliberationRequestIR {
                    schema: DELIBERATION_REQUEST_SCHEMA.to_string(),
                    request_id: "DELIBERATE-RAW-CHAIN".to_string(),
                    subject: "compose knowledge induced from raw state maps".to_string(),
                    evidence: vec![EvidenceIR {
                        evidence_id: "E-RAW-SEED".to_string(),
                        literal: literal("STATE::SEED"),
                        reliability_millis: 990,
                        source_ref: "public-raw:seed".to_string(),
                    }],
                    mechanisms: Vec::new(),
                    goals: vec![literal("STATE::FINAL")],
                    authority_envelope: AuthorityEnvelopeIR::default(),
                    immutable_constraints: Vec::new(),
                    max_depth: 4,
                    beam_width: 8,
                    max_hypotheses: 8,
                    max_counterfactuals: 8,
                },
                &MechanismQueryIR {
                    semantic_tags: vec!["raw-chain".to_string()],
                    known_proposition_ids: vec![
                        "STATE::SEED".to_string(),
                        "STATE::MIDDLE".to_string(),
                    ],
                    goal_proposition_ids: vec!["STATE::FINAL".to_string()],
                    max_results: 8,
                },
            )
            .unwrap();
        assert_eq!(result.recalled_mechanisms.len(), 2);
        assert_eq!(
            result.deliberation.disposition,
            DeliberationDispositionIR::GoalReachable
        );
        assert_eq!(
            result.deliberation.selected_plan.unwrap().mechanism_ids,
            ["M-RAW-CHAIN-A-B", "M-RAW-CHAIN-B-C"]
        );
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

    fn conversation_request(
        conversation_id: &str,
        turn_index: u64,
        text: &str,
    ) -> crate::conversation::ConversationTurnRequestIR {
        crate::conversation::ConversationTurnRequestIR {
            schema: crate::conversation::CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
            conversation_id: conversation_id.to_string(),
            turn_index,
            request_id: format!("{conversation_id}-{turn_index}"),
            modality: crate::conversation::ConversationInputModalityIR::Text,
            raw_text: text.to_string(),
            input_confidence_millis: 1_000,
            alternatives: Vec::new(),
            output_language: Some(LanguageCodeIR::Korean),
            context_tags: Vec::new(),
            max_plan_steps: 12,
        }
    }

    #[test]
    fn nested_possibility_does_not_become_an_implicit_delete_goal() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-MODAL-NONGOAL",
                1,
                "We might need to delete the cache.",
            ))
            .expect("modal turn");
        assert!(response.pragmatic_interpretation.inferred_goal.is_none());
        assert_eq!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .modal_scope_graph
                .root_world,
            crate::modality::ModalWorldIR::EpistemicPossible
        );
        assert!(response.conversation_state.active_goals.is_empty());
        assert_eq!(
            response.conversation_state.epistemic_ledger.records.len(),
            1
        );
        assert_eq!(
            response.conversation_state.epistemic_ledger.records[0]
                .signature
                .modal_world,
            crate::modality::ModalWorldIR::EpistemicPossible
        );
    }

    #[test]
    fn polite_modal_question_projects_the_action_but_not_modal_truth() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-POLITE-REQUEST",
                1,
                "Could you delete the cache?",
            ))
            .expect("polite request");
        let goal = response
            .pragmatic_interpretation
            .inferred_goal
            .as_ref()
            .expect("indirect request becomes a typed goal");
        assert_eq!(goal.intent, dockable_semantic_core::PlanIntentIR::Execute);
        assert!(goal.external_execution_authorized);
        assert_eq!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .modal_scope_graph
                .illocution,
            crate::modality::ModalIllocutionIR::PoliteRequest
        );
        assert!(
            !response
                .pragmatic_interpretation
                .compositional_analysis
                .modal_scope_graph
                .dialogue_truth_established
        );
    }

    #[test]
    fn conditional_directive_is_not_a_current_action_or_satisfied_condition() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-CONDITIONAL-AUTHORITY",
                1,
                "If the tests pass, deploy the service.",
            ))
            .expect("conditional directive");
        assert!(response.pragmatic_interpretation.inferred_goal.is_none());
        assert!(response.conversation_state.active_goals.is_empty());
        let conditional = &response
            .pragmatic_interpretation
            .compositional_analysis
            .modal_scope_graph
            .conditionals[0];
        assert!(!conditional.condition_satisfied);
        assert!(!conditional.external_execution_authorized);
        assert!(!conditional.reverse_inference_authorized);
        assert_eq!(response.conditional_guard_evaluations.len(), 1);
        assert_eq!(
            response.conditional_guard_evaluations[0].status,
            crate::conditional_guard::GuardStatusIR::Unresolved
        );
        assert!(response.grounded_response.is_none());
        assert!(response.output.grounded_plan_sha256.is_none());
    }

    #[test]
    fn later_actual_evidence_supports_guard_for_deliberation_only() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-SUPPORT",
            1,
            "If the tests pass, deploy the service.",
        ))
        .expect("guard declaration");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-SUPPORT",
                2,
                "The tests passed.",
            ))
            .expect("guard evidence");
        let evaluation = &response.conditional_guard_evaluations[0];
        assert_eq!(
            evaluation.status,
            crate::conditional_guard::GuardStatusIR::SupportedByDialogueEvidence
        );
        assert!(evaluation.deliberation_eligible);
        assert!(!evaluation.dialogue_truth_established);
        assert!(!evaluation.external_execution_authorized);
        assert!(!evaluation.reverse_inference_authorized);
        assert!(response.conversation_state.active_goals.is_empty());
        assert!(response.grounded_response.is_none());
        assert!(response.output.grounded_plan_sha256.is_none());
    }

    #[test]
    fn possible_evidence_and_observed_consequent_do_not_activate_guard() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-NO-REVERSE",
            1,
            "If the tests pass, deploy the service.",
        ))
        .expect("guard declaration");
        let possible = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-NO-REVERSE",
                2,
                "The tests might pass.",
            ))
            .expect("possible evidence");
        assert!(possible.conditional_guard_evaluations.is_empty());
        assert_eq!(
            possible.conversation_state.conditional_guard_store.guards[0].status,
            crate::conditional_guard::GuardStatusIR::Unresolved
        );
        let consequent = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-NO-REVERSE",
                3,
                "The service deployed.",
            ))
            .expect("consequent observation");
        let guard = &consequent.conversation_state.conditional_guard_store.guards[0];
        assert_eq!(
            guard.status,
            crate::conditional_guard::GuardStatusIR::Unresolved
        );
        assert!(!guard.reverse_inference_authorized);
        assert!(!guard.deliberation_eligible);
    }

    #[test]
    fn conflicting_sources_keep_guard_contested() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-CONFLICT",
            1,
            "If the tests pass, deploy the service.",
        ))
        .expect("guard declaration");
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-CONFLICT",
            2,
            "Alice says the tests passed.",
        ))
        .expect("supporting source");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-CONFLICT",
                3,
                "Bob says the tests failed.",
            ))
            .expect("contradicting source");
        let evaluation = &response.conditional_guard_evaluations[0];
        assert_eq!(
            evaluation.status,
            crate::conditional_guard::GuardStatusIR::Contested
        );
        assert_eq!(evaluation.evidence.len(), 2);
        assert!(!evaluation.deliberation_eligible);
        assert!(!evaluation.external_execution_authorized);
    }

    #[test]
    fn korean_unless_guard_uses_negated_antecedent_without_auto_action() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-KOREAN",
            1,
            "테스트가 통과하지 않으면 배포를 멈춰.",
        ))
        .expect("Korean unless guard");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-KOREAN",
                2,
                "테스트가 실패했다.",
            ))
            .expect("Korean guard evidence");
        let evaluation = &response.conditional_guard_evaluations[0];
        assert_eq!(
            evaluation.status,
            crate::conditional_guard::GuardStatusIR::SupportedByDialogueEvidence
        );
        assert!(evaluation.realized_text.contains("자동 실행 권한은 없어"));
        assert!(response.conversation_state.active_goals.is_empty());
    }

    #[test]
    fn recognized_question_does_not_mutate_existing_guard_state() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-QUESTION",
            1,
            "If the tests pass, deploy the service.",
        ))
        .expect("guard declaration");
        let supported = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-QUESTION",
                2,
                "The tests passed.",
            ))
            .expect("guard support");
        let before = supported.conversation_state.conditional_guard_store.clone();
        let question = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-QUESTION",
                3,
                "Is it true that the tests passed?",
            ))
            .expect("recognized discourse question");
        assert!(question.discourse_answer.is_some());
        assert!(question.conditional_guard_evaluations.is_empty());
        assert_eq!(question.conversation_state.conditional_guard_store, before);
    }

    #[test]
    fn discourse_question_answers_from_prior_attribution_without_making_a_plan() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-SOURCE",
            1,
            "Alice says that the server is down.",
        ))
        .expect("attributed statement");
        let mut question = conversation_request("CHAT-QA-SOURCE", 2, "What did Alice say?");
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("discourse answer");
        let answer = response
            .discourse_answer
            .as_ref()
            .expect("typed discourse answer");
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
        );
        assert_eq!(answer.evidence[0].source_actor, "alice");
        assert!(response.grounded_response.is_none());
        assert!(response.conversation_state.active_goals.is_empty());
        assert_eq!(
            response.conversation_state.epistemic_ledger.records.len(),
            1
        );
        assert!(response.pragmatic_state.task_frames.is_empty());
        assert_eq!(response.output.unsupported_freeform_claims, 0);
    }

    #[test]
    fn modal_status_question_preserves_possible_world_and_record_count() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-MODAL",
            1,
            "Alice believes that the server might be down.",
        ))
        .expect("modal belief");
        let mut question = conversation_request(
            "CHAT-QA-MODAL",
            2,
            "Is the server merely possible or actual?",
        );
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("modal answer");
        let answer = response.discourse_answer.expect("typed modal answer");
        assert_eq!(answer.evidence.len(), 1);
        assert_eq!(
            answer.evidence[0].modal_world,
            crate::modality::ModalWorldIR::EpistemicPossible
        );
        assert_eq!(
            response.conversation_state.epistemic_ledger.records.len(),
            1
        );
        assert!(!answer.dialogue_truth_established);
    }

    #[test]
    fn conflicting_sources_are_answered_without_selecting_a_truth_winner() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-CONFLICT",
            1,
            "Alice says that the server is up.",
        ))
        .expect("Alice record");
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-CONFLICT",
            2,
            "Bob says that the server is down.",
        ))
        .expect("Bob record");
        let mut question = conversation_request(
            "CHAT-QA-CONFLICT",
            3,
            "Are Alice and Bob in conflict about the server?",
        );
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("conflict answer");
        let answer = response.discourse_answer.expect("typed conflict answer");
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::ConflictingDialogueRecords
        );
        assert_eq!(answer.evidence.len(), 2);
        assert!(answer.realized_text.contains("No source has been selected"));
    }

    #[test]
    fn factive_attribution_answer_remains_presented_as_known_not_fact() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-FACTIVE",
            1,
            "Alice knows that the server is down.",
        ))
        .expect("factive attribution");
        let mut question = conversation_request("CHAT-QA-FACTIVE", 2, "What does Alice know?");
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("factive answer");
        let answer = response.discourse_answer.expect("typed factive answer");
        assert_eq!(
            answer.evidence[0].epistemic_status,
            crate::attribution::EpistemicStatusIR::PresentedAsKnown
        );
        assert!(!answer.evidence[0].dialogue_truth_established);
        assert!(answer.realized_text.contains("not established facts"));
    }

    #[test]
    fn presuppositional_why_question_abstains_and_does_not_add_a_belief() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-PRESUPPOSITION",
            1,
            "Alice believes that the server might fail.",
        ))
        .expect("possible failure belief");
        let mut question =
            conversation_request("CHAT-QA-PRESUPPOSITION", 2, "Why did the server fail?");
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("presupposition response");
        let answer = response
            .discourse_answer
            .expect("typed presupposition response");
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::PresuppositionUnverified
        );
        assert_eq!(
            response.conversation_state.epistemic_ledger.records.len(),
            1
        );
        assert!(response.conversation_state.active_goals.is_empty());
    }

    #[test]
    fn korean_source_question_uses_the_same_typed_answer_path() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-KOREAN",
            1,
            "민수는 서버가 다운됐다고 말했다.",
        ))
        .expect("Korean attribution");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QA-KOREAN",
                2,
                "민수는 뭐라고 말했어?",
            ))
            .expect("Korean source answer");
        let answer = response.discourse_answer.expect("typed Korean answer");
        assert_eq!(answer.language, LanguageCodeIR::Korean);
        assert_eq!(answer.evidence.len(), 1);
        assert!(answer.realized_text.contains("사실 확정은 아니야"));
        assert_eq!(response.output.unsupported_freeform_claims, 0);
    }

    #[test]
    fn actuality_question_treats_expletive_it_and_complementizer_that_as_syntax() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-SYNTACTIC-REFERENCE",
            1,
            "Alice says that the server is down.",
        ))
        .expect("attributed statement");
        let mut question = conversation_request(
            "CHAT-QA-SYNTACTIC-REFERENCE",
            2,
            "Is it true that the server is down?",
        );
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("actuality answer");
        let answer = response.discourse_answer.expect("typed actuality answer");
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::DialogueTruthNotEstablished
        );
        assert_eq!(answer.evidence.len(), 1);
        assert_eq!(response.conversation_state.unresolved_reference_count, 0);
        assert!(response.conversation_state.active_goals.is_empty());
    }

    #[test]
    fn attribution_attitude_projects_desired_and_predicted_worlds() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let wanted = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QA-ATTITUDE-WORLD",
                1,
                "Leo wants the team to retry.",
            ))
            .expect("desired attribution");
        assert_eq!(
            wanted.conversation_state.epistemic_ledger.records[0]
                .signature
                .modal_world,
            crate::modality::ModalWorldIR::Desired
        );
        let expected = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QA-ATTITUDE-WORLD",
                2,
                "Uma expects that the rollout will finish.",
            ))
            .expect("expected attribution");
        assert!(
            expected
                .conversation_state
                .epistemic_ledger
                .records
                .iter()
                .any(|record| record.signature.modal_world
                    == crate::modality::ModalWorldIR::Predicted)
        );
    }

    #[test]
    fn temporal_question_uses_event_time_not_report_turn() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let statement = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TEMPORAL-TIME",
                1,
                "The backup completed yesterday.",
            ))
            .expect("temporal statement");
        assert_eq!(statement.conversation_state.temporal_graph.events.len(), 1);
        assert_eq!(
            statement.conversation_state.temporal_graph.events[0]
                .event_time
                .as_ref()
                .and_then(|time| time.relative_day_offset),
            Some(-1)
        );
        let mut question =
            conversation_request("CHAT-TEMPORAL-TIME", 2, "When did the backup complete?");
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("temporal answer");
        let answer = response.temporal_answer.expect("typed temporal answer");
        assert_eq!(
            answer.disposition,
            crate::temporal::TemporalAnswerDispositionIR::AnsweredFromTemporalGraph
        );
        assert!(answer.realized_text.contains("DAY_OFFSET:-1"));
        assert!(response.grounded_response.is_none());
        assert!(response.conversation_state.active_goals.is_empty());
        assert_eq!(response.conversation_state.temporal_graph.events.len(), 1);
    }

    #[test]
    fn temporal_before_question_cites_relation_without_asserting_truth() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-TEMPORAL-BEFORE",
            1,
            "The backup completed before the deploy started.",
        ))
        .expect("temporal relation");
        let mut question = conversation_request(
            "CHAT-TEMPORAL-BEFORE",
            2,
            "What happened before the deploy started?",
        );
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("temporal relation answer");
        let answer = response.temporal_answer.expect("typed temporal answer");
        assert_eq!(answer.relation_evidence.len(), 1);
        assert!(answer
            .event_evidence
            .iter()
            .any(|event| event.surface.contains("backup")));
        assert!(!answer.dialogue_truth_established);
        assert!(!answer.external_execution_authorized);
    }

    #[test]
    fn temporal_deictic_chain_supports_transitive_multi_turn_reasoning() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-TEMPORAL-CHAIN",
            1,
            "The backup completed.",
        ))
        .expect("first event");
        let second = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TEMPORAL-CHAIN",
                2,
                "After that, the deploy started.",
            ))
            .expect("second event");
        assert_eq!(second.conversation_state.unresolved_reference_count, 0);
        let third = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TEMPORAL-CHAIN",
                3,
                "After that, the monitor failed.",
            ))
            .expect("third event");
        assert_eq!(third.conversation_state.temporal_graph.events.len(), 3);
        assert_eq!(third.conversation_state.temporal_graph.relations.len(), 2);
        let mut question = conversation_request(
            "CHAT-TEMPORAL-CHAIN",
            4,
            "Did the backup complete before the monitor failed?",
        );
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("transitive answer");
        let answer = response.temporal_answer.expect("typed temporal answer");
        assert_eq!(
            answer.disposition,
            crate::temporal::TemporalAnswerDispositionIR::AnsweredByTransitivePath
        );
        assert_eq!(answer.relation_evidence.len(), 2);
    }

    #[test]
    fn korean_temporal_relation_uses_same_graph_path() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-TEMPORAL-KOREAN",
            1,
            "배포가 시작되기 전에 백업이 완료됐다.",
        ))
        .expect("Korean temporal relation");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TEMPORAL-KOREAN",
                2,
                "배포가 시작되기 전에 무슨 일이 있었어?",
            ))
            .expect("Korean temporal answer");
        let answer = response.temporal_answer.expect("typed temporal answer");
        assert_eq!(answer.language, LanguageCodeIR::Korean);
        assert_eq!(answer.relation_evidence.len(), 1);
        assert!(answer
            .event_evidence
            .iter()
            .any(|event| event.surface.contains("백업")));
    }

    #[test]
    fn conversational_frontend_repairs_surface_noise_before_semantic_planning() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-NOISE",
                1,
                "음... 파일 오류를 고처줘",
            ))
            .expect("conversation turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::Grounded
        );
        assert_eq!(response.normalization.semantic_text, "파일 오류를 고쳐줘");
        let grounded = response.grounded_response.expect("grounded response");
        assert_eq!(
            grounded.understanding.intent,
            dockable_semantic_core::PlanIntentIR::Repair
        );
        assert_eq!(
            grounded.understanding.original_text,
            "음... 파일 오류를 고처줘"
        );
        assert_eq!(
            response.output.grounded_plan_sha256,
            Some(grounded.plan.plan_sha256.clone())
        );
        assert!(response.output.text.starts_with("알겠어."));
        assert_eq!(response.output.unsupported_freeform_claims, 0);
    }

    #[test]
    fn conversational_reference_uses_prior_turn_without_rewriting_the_world_model() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request("CHAT-REF", 1, "파일을 열어"))
            .expect("first turn");
        let response = api
            .process_conversation_turn(&conversation_request("CHAT-REF", 2, "음... 그걸 확인해"))
            .expect("second turn");
        assert_eq!(response.reference_resolution.resolved_reference_count, 1);
        assert_eq!(
            response.reference_resolution.resolved_semantic_text,
            "파일을 확인해"
        );
        assert_eq!(response.conversation_state.completed_turns, 2);
        assert_eq!(response.conversation_state.active_referents.len(), 1);
        assert_eq!(
            response.conversation_state.active_referents[0].canonical_concept,
            "C_OBJECT_FILE"
        );
    }

    #[test]
    fn filler_only_turn_is_acknowledged_without_a_fake_plan() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request("CHAT-HOLD", 1, "음..."))
            .expect("hold turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::HoldFloor
        );
        assert!(response.grounded_response.is_none());
        assert!(response.output.grounded_plan_sha256.is_none());
        assert_eq!(response.output.text, "응, 천천히 말해. 듣고 있어.");
    }

    #[test]
    fn greeting_receives_a_social_response_without_planning() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request("CHAT-HELLO", 1, "안녕"))
            .expect("greeting");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::BackchannelOnly
        );
        assert!(response.grounded_response.is_none());
        assert_eq!(response.output.text, "안녕! 무엇을 도와줄까?");
    }

    #[test]
    fn common_action_language_selects_execute_intent() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request("CHAT-OPEN", 1, "파일을 열어"))
            .expect("execute request");
        assert_eq!(
            response
                .grounded_response
                .expect("grounded response")
                .understanding
                .intent,
            dockable_semantic_core::PlanIntentIR::Execute
        );
    }

    #[test]
    fn cross_turn_parallel_ellipsis_reuses_the_typed_action_with_a_new_subject() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-PARALLEL-ELLIPSIS",
            1,
            "파일을 확인해",
        ))
        .expect("first turn");
        let response = api
            .process_conversation_turn(&conversation_request("CHAT-PARALLEL-ELLIPSIS", 2, "문서도"))
            .expect("elliptical second turn");
        assert_eq!(
            response.reference_resolution.resolved_semantic_text,
            "문서를 확인해"
        );
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate()
            .expect("inherited action is grounded again");
        assert_eq!(selected.subject, "문서");
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            crate::conversation::DiscourseBindingKindIR::EllipticalAction
        );
    }

    #[test]
    fn cross_turn_argument_correction_revises_the_prior_goal_not_the_world_model() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ARGUMENT-CORRECTION",
                1,
                "파일을 열어",
            ))
            .expect("first turn");
        assert_eq!(
            first
                .pragmatic_interpretation
                .compositional_analysis
                .selected_candidates()
                .len(),
            1
        );
        assert_eq!(first.conversation_state.active_goals.len(), 1);
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ARGUMENT-CORRECTION",
                2,
                "그거 말고 폴더로",
            ))
            .expect("correction turn");
        assert_eq!(
            response.reference_resolution.resolved_semantic_text,
            "폴더를 열어"
        );
        assert_eq!(
            response
                .reference_resolution
                .discourse_bindings
                .last()
                .unwrap()
                .kind,
            crate::conversation::DiscourseBindingKindIR::CorrectedArgument
        );
        assert_eq!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .selected_candidate()
                .expect("corrected goal")
                .subject,
            "폴더"
        );
    }

    #[test]
    fn cross_language_pronoun_resolution_feeds_the_pragmatic_parser() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-CROSS-LANGUAGE-REFERENCE",
            1,
            "파일을 열어",
        ))
        .expect("Korean first turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-CROSS-LANGUAGE-REFERENCE",
                2,
                "fix it",
            ))
            .expect("English reference turn");
        assert_eq!(
            response.reference_resolution.resolved_semantic_text,
            "fix file"
        );
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate()
            .expect("resolved English action");
        assert_eq!(selected.subject, "file");
        assert_eq!(
            selected.intent,
            dockable_semantic_core::PlanIntentIR::Repair
        );
    }

    #[test]
    fn ambiguous_repeat_of_a_multi_goal_program_requires_clarification() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-AMBIGUOUS-REPEAT",
            1,
            "파일을 읽고 저장해",
        ))
        .expect("multi-goal first turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-AMBIGUOUS-REPEAT",
                2,
                "그대로 해",
            ))
            .expect("ambiguous repeat turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(response.grounded_response.is_none());
        assert_eq!(
            response.reference_resolution.ambiguous_reference_surfaces,
            vec!["ELLIPTICAL_ACTION"]
        );
    }

    #[test]
    fn event_reference_is_reintroduced_as_quoted_non_authoritative_content() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-EVENT-REFERENCE",
            1,
            "보고서를 저장해",
        ))
        .expect("event source turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-EVENT-REFERENCE",
                2,
                "그 작업을 설명해",
            ))
            .expect("event reference turn");
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            crate::conversation::DiscourseBindingKindIR::EventReference
        );
        assert!(response
            .reference_resolution
            .resolved_semantic_text
            .contains("‘보고서를 저장해’"));
        let analysis = &response.pragmatic_interpretation.compositional_analysis;
        assert_eq!(
            analysis
                .selected_candidate()
                .expect("outer explanation")
                .intent,
            dockable_semantic_core::PlanIntentIR::Explain
        );
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == dockable_semantic_core::PlanIntentIR::Execute
                && candidate.disposition
                    == crate::compositional_semantics::CandidateDispositionIR::NonAuthoritativeMention
        }));
    }

    #[test]
    fn result_reference_keeps_provenance_across_an_english_turn() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-RESULT-REFERENCE",
            1,
            "save report",
        ))
        .expect("result source turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-RESULT-REFERENCE",
                2,
                "explain that result",
            ))
            .expect("result reference turn");
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            crate::conversation::DiscourseBindingKindIR::ResultReference
        );
        assert_eq!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .selected_candidate()
                .expect("explanation")
                .intent,
            dockable_semantic_core::PlanIntentIR::Explain
        );
    }

    #[test]
    fn proposition_reference_uses_prior_assertion_without_inventing_authority() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROPOSITION-REFERENCE",
                1,
                "빌드가 실패했다",
            ))
            .expect("assertion turn");
        assert!(first
            .conversation_state
            .active_discourse_referents
            .iter()
            .any(|referent| {
                referent.kind == crate::conversation::DiscourseReferentKindIR::Proposition
                    && !referent.external_execution_authorized
            }));
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROPOSITION-REFERENCE",
                2,
                "그 사실을 설명해",
            ))
            .expect("proposition reference turn");
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            crate::conversation::DiscourseBindingKindIR::PropositionReference
        );
        assert!(response
            .reference_resolution
            .resolved_semantic_text
            .contains("‘빌드가 실패했다’"));
    }

    #[test]
    fn attributed_command_is_remembered_as_a_claim_not_reissued_as_a_goal() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ATTRIBUTED-COMMAND",
                1,
                "Alice said delete the file",
            ))
            .expect("attributed command turn");
        assert!(response.pragmatic_interpretation.inferred_goal.is_none());
        let analysis = &response.pragmatic_interpretation.compositional_analysis;
        assert!(analysis.attribution_graph.validate());
        assert!(analysis.candidates.iter().all(|candidate| {
            !analysis
                .attribution_graph
                .attributes_frame(&candidate.source_frame_id)
                || !candidate.external_execution_authorized
        }));
        let proposition = response
            .conversation_state
            .active_discourse_referents
            .iter()
            .find(|referent| {
                referent.kind == crate::conversation::DiscourseReferentKindIR::Proposition
            })
            .expect("attributed proposition memory");
        assert_eq!(proposition.attributed_source.as_deref(), Some("alice"));
        assert_eq!(
            proposition.attribution_attitude,
            Some(crate::attribution::AttributionAttitudeIR::Say)
        );
        assert!(!proposition.external_execution_authorized);
        let grounded = response
            .grounded_response
            .expect("grounded attribution response");
        assert!(grounded
            .understanding
            .semantic_tags
            .contains(&"attributed_truth_not_established".to_string()));
    }

    #[test]
    fn nested_attribution_can_be_recalled_by_source_without_collapsing_beliefs() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-NESTED-ATTRIBUTION",
                1,
                "Alice says Bob believes that the server is down",
            ))
            .expect("nested attribution turn");
        assert_eq!(first.disposition, ConversationTurnDispositionIR::Grounded);
        assert_eq!(
            first
                .conversation_state
                .active_discourse_referents
                .iter()
                .filter(|referent| {
                    referent.kind == crate::conversation::DiscourseReferentKindIR::Proposition
                })
                .count(),
            2
        );
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-NESTED-ATTRIBUTION",
                2,
                "explain Bob's belief",
            ))
            .expect("source-specific reference");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::Grounded
        );
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            crate::conversation::DiscourseBindingKindIR::PropositionReference
        );
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].referent_ids,
            vec!["DREF-P-000001-02"]
        );
        assert!(response
            .reference_resolution
            .resolved_semantic_text
            .contains("‘the server is down’"));
    }

    #[test]
    fn same_source_explicit_update_supersedes_prior_belief_without_becoming_fact() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-BELIEF-UPDATE",
            1,
            "Alice says that the server is down",
        ))
        .expect("initial attributed state");
        let updated = api
            .process_conversation_turn(&conversation_request(
                "CHAT-BELIEF-UPDATE",
                2,
                "Alice now says that the server is up",
            ))
            .expect("updated attributed state");
        let ledger = &updated.conversation_state.epistemic_ledger;
        assert_eq!(ledger.records.len(), 2);
        assert_eq!(
            ledger.records[0].status,
            crate::epistemic::BeliefRecordStatusIR::Superseded
        );
        assert_eq!(
            ledger.records[1].status,
            crate::epistemic::BeliefRecordStatusIR::Active
        );
        assert!(ledger.revisions.iter().any(|revision| {
            revision.kind == crate::epistemic::BeliefRevisionKindIR::Supersedes
        }));
        assert!(ledger.records.iter().all(|record| {
            !record.dialogue_truth_established && !record.external_execution_authorized
        }));
    }

    #[test]
    fn different_sources_with_opposite_states_remain_contested() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-BELIEF-CONFLICT",
            1,
            "Alice says that the server is down",
        ))
        .expect("Alice claim");
        let conflict = api
            .process_conversation_turn(&conversation_request(
                "CHAT-BELIEF-CONFLICT",
                2,
                "Bob says that the server is up",
            ))
            .expect("Bob claim");
        let ledger = &conflict.conversation_state.epistemic_ledger;
        assert_eq!(ledger.records.len(), 2);
        assert!(ledger
            .records
            .iter()
            .all(|record| { record.status == crate::epistemic::BeliefRecordStatusIR::Contested }));
        assert_eq!(
            conflict
                .conversation_state
                .active_discourse_referents
                .iter()
                .filter(|referent| {
                    referent.kind == crate::conversation::DiscourseReferentKindIR::Proposition
                })
                .count(),
            2
        );
    }

    #[test]
    fn explicit_claim_retraction_removes_it_from_active_reference_state() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-BELIEF-RETRACT",
            1,
            "Alice claims that the cache is corrupt",
        ))
        .expect("claim");
        let retracted = api
            .process_conversation_turn(&conversation_request(
                "CHAT-BELIEF-RETRACT",
                2,
                "Alice retracts that claim",
            ))
            .expect("retraction");
        assert_eq!(
            retracted.conversation_state.epistemic_ledger.records[0].status,
            crate::epistemic::BeliefRecordStatusIR::Retracted
        );
        assert!(retracted
            .conversation_state
            .active_discourse_referents
            .iter()
            .all(|referent| {
                referent.kind != crate::conversation::DiscourseReferentKindIR::Proposition
            }));
        assert!(retracted
            .conversation_state
            .epistemic_ledger
            .revisions
            .iter()
            .any(|revision| { revision.kind == crate::epistemic::BeliefRevisionKindIR::Retracts }));
    }

    #[test]
    fn competing_prior_propositions_require_clarification() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-PROPOSITION-AMBIGUITY",
            1,
            "빌드가 실패했다. 로그가 비었다.",
        ))
        .expect("two proposition turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROPOSITION-AMBIGUITY",
                2,
                "그 사실을 설명해",
            ))
            .expect("ambiguous reference turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(response
            .reference_resolution
            .ambiguous_reference_surfaces
            .contains(&"Proposition_REFERENCE".to_string()));
    }

    #[test]
    fn semantic_roles_and_quantifier_scope_reach_goal_ir_constraints() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ROLE-GOAL-IR",
                1,
                "서버에서 모든 파일을 읽어",
            ))
            .expect("role-grounded turn");
        let grounded = response.grounded_response.expect("grounded response");
        assert!(grounded
            .understanding
            .semantic_tags
            .contains(&"semantic_role_graph".to_string()));
        assert!(grounded
            .understanding
            .semantic_tags
            .contains(&"quantifier_scope_explicit".to_string()));
        assert!(grounded
            .understanding
            .constraints
            .iter()
            .any(|constraint| constraint.contains("Source=서버")
                && constraint.contains("Theme=모든 파일")));
        assert!(grounded
            .understanding
            .constraints
            .iter()
            .any(|constraint| constraint.contains("All(파일)")));
    }

    #[test]
    fn indirect_cost_benefit_utterance_becomes_a_verified_continuation_gate() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PRAGMATIC-GATE",
                1,
                "유일하게 고통을 참고 진행하려면 기존에는 점수만 높았지 실제 코딩능력은 한참 낮았다. 왜냐? capability와 routing이 결합되어 나온 거품점수라서 실제 커버리지는 낮았다. 그래서 통합을 하면 커버리지를 확장하는 효과가 있다. 이러면 할만하지",
            ))
            .expect("pragmatic turn");
        assert!(response
            .normalization
            .operations
            .iter()
            .all(|operation| operation.before != "실제"));
        assert!(response.pragmatic_interpretation.clauses.len() >= 5);
        assert_eq!(
            response.pragmatic_interpretation.speech_act,
            crate::pragmatics::SpeechActIR::ConditionalContinuation
        );
        let gate = response
            .pragmatic_interpretation
            .continuation_gate
            .as_ref()
            .expect("continuation gate");
        assert_eq!(gate.current_task, "통합");
        assert_eq!(gate.required_benefit, "커버리지를 확장하는 효과가 있다");
        assert_eq!(
            gate.negative_action,
            crate::pragmatics::DecisionBranchActionIR::ReportNegativeAndAskWhetherToStop
        );
        let grounded = response.grounded_response.expect("grounded response");
        assert_eq!(
            grounded.understanding.intent,
            dockable_semantic_core::PlanIntentIR::Investigate
        );
        assert!(grounded
            .understanding
            .subject
            .starts_with("continuation_gate(task=통합"));
        assert!(response.output.text.contains("이득이 확인되면 계속"));
        assert!(response.output.text.contains("멈출지 물을게"));
    }

    #[test]
    fn standalone_language_api_uses_the_same_pragmatic_reasoning_circuit() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process(&NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "PRAGMATIC-STANDALONE".to_string(),
                text: "리팩터링은 힘들다. 리팩터링을 하면 장애가 줄어드는 효과가 있다. 그 정도 이득이면 계속 진행할 만하다.".to_string(),
                output_language: Some(LanguageCodeIR::Korean),
                context_tags: Vec::new(),
                max_plan_steps: 12,
            })
            .expect("standalone pragmatic reasoning");
        assert!(response.pragmatic_interpretation.has_continuation_gate());
        assert_eq!(
            response.understanding.intent,
            dockable_semantic_core::PlanIntentIR::Investigate
        );
        assert_ne!(
            response.understanding.subject,
            response.understanding.original_text
        );
    }

    #[test]
    fn indirect_problem_statement_projects_a_repair_goal_without_broad_authority() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-IMPLICIT-REPAIR",
                1,
                "배포 후 오류가 늘었네. 이 상태로 둘 수는 없지.",
            ))
            .expect("implicit repair");
        let goal = response
            .pragmatic_interpretation
            .inferred_goal
            .as_ref()
            .expect("inferred goal");
        assert_eq!(goal.intent, dockable_semantic_core::PlanIntentIR::Repair);
        assert!(!goal.external_execution_authorized);
        assert!(response.output.text.contains("수리가 필요"));
        assert_eq!(
            response
                .grounded_response
                .expect("grounded response")
                .understanding
                .intent,
            dockable_semantic_core::PlanIntentIR::Repair
        );
    }

    #[test]
    fn pragmatic_memory_restores_an_elliptical_continuation_gate_across_turns() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-LONG-PRAGMATIC",
                1,
                "마이그레이션은 힘들다. 마이그레이션을 하면 장애 빈도가 감소한다. 그 정도 이득이면 계속 진행할 만하다.",
            ))
            .expect("first gate");
        assert!(first.pragmatic_state.pending_continuation_gate.is_some());
        let second = api
            .process_conversation_turn(&conversation_request(
                "CHAT-LONG-PRAGMATIC",
                2,
                "그 정도면 계속할 만하지",
            ))
            .expect("elliptical gate");
        let gate = second
            .pragmatic_interpretation
            .continuation_gate
            .as_ref()
            .expect("restored gate");
        assert_eq!(gate.current_task, "마이그레이션");
        assert!(gate.required_benefit.contains("장애 빈도"));
        assert_eq!(second.pragmatic_state.completed_turns, 2);
    }

    #[test]
    fn user_rejection_suspends_a_pending_gate_until_explicitly_reopened() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-SUSPEND-GATE",
            1,
            "리팩터링은 힘들다. 리팩터링을 하면 장애가 줄어든다. 이러면 계속할 만하다.",
        ))
        .expect("initial gate");
        let rejection = api
            .process_conversation_turn(&conversation_request(
                "CHAT-SUSPEND-GATE",
                2,
                "그래도 계속하지 마",
            ))
            .expect("rejection");
        assert_eq!(
            rejection
                .pragmatic_state
                .pending_continuation_gate
                .as_ref()
                .expect("remembered gate")
                .status,
            crate::pragmatic_memory::PendingGateStatusIR::SuspendedByUser
        );
        let elliptical = api
            .process_conversation_turn(&conversation_request(
                "CHAT-SUSPEND-GATE",
                3,
                "그 정도면 계속할 만하지",
            ))
            .expect("later ellipsis");
        assert!(elliptical
            .pragmatic_interpretation
            .continuation_gate
            .is_none());
    }

    #[test]
    fn sarcastic_praise_after_failure_is_not_treated_as_approval() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-SARCASM",
                1,
                "테스트가 전부 깨졌네. 아주 잘했어.",
            ))
            .expect("sarcastic turn");
        assert_eq!(
            response.pragmatic_interpretation.speech_act,
            crate::pragmatics::SpeechActIR::NegativeEvaluation
        );
        assert!(
            response
                .pragmatic_interpretation
                .nonliteral_analysis
                .literal_execution_blocked
        );
        assert!(response.output.text.contains("긍정 승인이 아니라"));
    }

    #[test]
    fn context_free_literal_or_metaphorical_fire_fails_closed() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-FIRE-AMBIGUITY",
                1,
                "여기 불이 났어",
            ))
            .expect("ambiguous fire");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(response.grounded_response.is_none());
        assert!(response.output.text.contains("문자 그대로의 상황"));
    }

    #[test]
    fn software_metaphor_selects_problem_state_without_literal_execution() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-SOFTWARE-METAPHOR",
                1,
                "배포 뒤 프로젝트에 불이 났어",
            ))
            .expect("software metaphor");
        assert_eq!(
            response
                .pragmatic_interpretation
                .nonliteral_analysis
                .expressions[0]
                .selected_reading,
            crate::nonliteral::ReadingSelectionIR::Figurative
        );
        assert!(response.output.text.contains("비유적 상태"));
    }

    #[test]
    fn compositional_scope_selects_explanation_and_blocks_negated_repair() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-COMPOSITIONAL-NEGATION",
                1,
                "서비스를 수정하지 말고 장애 원인만 설명해줘",
            ))
            .expect("scope-aware turn");
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate()
            .expect("outer explanation");
        assert_eq!(
            selected.intent,
            dockable_semantic_core::PlanIntentIR::Explain
        );
        assert!(response
            .pragmatic_interpretation
            .compositional_analysis
            .candidates
            .iter()
            .any(|candidate| {
                candidate.intent == dockable_semantic_core::PlanIntentIR::Repair
                    && candidate.disposition
                        == crate::compositional_semantics::CandidateDispositionIR::BlockedByNegation
            }));
        assert_eq!(
            response
                .grounded_response
                .expect("grounded explanation")
                .understanding
                .intent,
            dockable_semantic_core::PlanIntentIR::Explain
        );
    }

    #[test]
    fn quoted_destructive_command_remains_non_authoritative_in_cognitive_path() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-COMPOSITIONAL-QUOTE",
                1,
                "‘데이터를 삭제해’라는 문장을 설명해",
            ))
            .expect("quoted command turn");
        let analysis = &response.pragmatic_interpretation.compositional_analysis;
        assert_eq!(
            analysis.selected_candidate().expect("outer request").intent,
            dockable_semantic_core::PlanIntentIR::Explain
        );
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == dockable_semantic_core::PlanIntentIR::Execute
                && candidate.disposition
                    == crate::compositional_semantics::CandidateDispositionIR::NonAuthoritativeMention
                && !candidate.external_execution_authorized
        }));
    }

    #[test]
    fn equally_supported_conflicting_requests_require_clarification() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-COMPOSITIONAL-COMPETITION",
                1,
                "파일을 분석해; 코드를 수정해",
            ))
            .expect("competing request turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(response.grounded_response.is_none());
        assert!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .clarification_required
        );
        assert!(response.output.text.contains("실제 요청"));
    }

    #[test]
    fn injected_predicate_enters_the_same_compositional_reasoning_path() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        assert!(api
            .inject_compositional_predicate(crate::compositional_semantics::PredicateLexemeIR {
                schema: crate::compositional_semantics::PREDICATE_LEXEME_SCHEMA.to_string(),
                predicate_id: "P-REFINE-DOCUMENT-COGNITIVE".to_string(),
                language: LanguageCodeIR::Korean,
                surface_forms: vec!["다듬".to_string()],
                canonical_predicate: "C_REFINE_DOCUMENT".to_string(),
                intent_hint: dockable_semantic_core::PlanIntentIR::Create,
                definition: "revise a document into a clearer finished form".to_string(),
                confidence_millis: 920,
            })
            .expect("inject predicate"));
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-LEARNED-PREDICATE",
                1,
                "문서를 다듬어줘",
            ))
            .expect("learned predicate turn");
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate()
            .expect("selected learned predicate");
        assert_eq!(
            selected.intent,
            dockable_semantic_core::PlanIntentIR::Create
        );
        assert_eq!(selected.subject, "문서");
        assert_eq!(
            response
                .grounded_response
                .expect("grounded create response")
                .understanding
                .intent,
            dockable_semantic_core::PlanIntentIR::Create
        );
    }

    #[test]
    fn learned_predicate_snapshot_restores_language_capability() {
        let predicate = crate::compositional_semantics::PredicateLexemeIR {
            schema: crate::compositional_semantics::PREDICATE_LEXEME_SCHEMA.to_string(),
            predicate_id: "P-PERSIST-REFINE-COGNITIVE".to_string(),
            language: LanguageCodeIR::Korean,
            surface_forms: vec!["다듬".to_string()],
            canonical_predicate: "C_REFINE_DOCUMENT".to_string(),
            intent_hint: dockable_semantic_core::PlanIntentIR::Create,
            definition: "revise a document into a clearer finished form".to_string(),
            confidence_millis: 920,
        };
        let mut source = CognitiveApi::new_embedded().unwrap();
        source
            .inject_compositional_predicate(predicate)
            .expect("inject predicate");
        let snapshot = source.export_compositional_predicates();

        let mut restored = CognitiveApi::new_embedded().unwrap();
        restored
            .import_compositional_predicates(&snapshot)
            .expect("restore predicates");
        let response = restored
            .process_conversation_turn(&conversation_request(
                "CHAT-RESTORED-PREDICATE",
                1,
                "문서를 다듬어줘",
            ))
            .expect("restored language capability");
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate()
            .expect("restored predicate selected");
        assert_eq!(
            selected.intent,
            dockable_semantic_core::PlanIntentIR::Create
        );
        assert_eq!(selected.subject, "문서");
    }

    #[test]
    fn ordered_multi_goal_language_projects_a_plan_without_losing_prohibition() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ORDERED-GOALS",
                1,
                "파일을 읽고 각 줄을 변환한 뒤 저장해. 원본은 지우지 마",
            ))
            .expect("ordered multi-goal turn");
        let graph = response
            .pragmatic_interpretation
            .compositional_analysis
            .goal_graph
            .as_ref()
            .expect("goal graph");
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.prohibitions.len(), 1);
        let grounded = response.grounded_response.expect("grounded plan");
        assert_eq!(
            grounded.understanding.intent,
            dockable_semantic_core::PlanIntentIR::Plan
        );
        assert_eq!(grounded.understanding.desired_outcomes.len(), 3);
        assert!(grounded
            .understanding
            .constraints
            .iter()
            .any(|constraint| constraint.contains("preserve explicit prohibition")));
        assert!(grounded
            .understanding
            .semantic_tags
            .contains(&"ordered_multi_goal_request".to_string()));
    }

    #[test]
    fn ambiguous_voice_turn_asks_instead_of_guessing() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut request = conversation_request("CHAT-VOICE", 1, "파일을 열어");
        request.modality = crate::conversation::ConversationInputModalityIR::VoiceTranscript;
        request.input_confidence_millis = 800;
        request.alternatives = vec![crate::conversation::UtteranceAlternativeIR {
            text: "파일을 얼어".to_string(),
            confidence_millis: 770,
        }];
        let response = api.process_conversation_turn(&request).expect("voice turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(response.grounded_response.is_none());
        assert!(response.output.text.contains("어느 쪽인지"));
        assert_eq!(response.conversation_state.unresolved_reference_count, 1);
    }
}
