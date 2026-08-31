//! Adapters are deliberately outside `dockable-semantic-core`.

pub mod attribution;
pub mod cognitive;
pub mod compositional_semantics;
pub mod conditional_guard;
pub mod conversation;
pub mod discourse_qa;
mod document_design;
pub mod document_swarm;
pub mod epistemic;
pub mod generic;
pub mod knowledge_work;
pub mod language;
pub mod language_knowledge;
pub mod lexical_memory;
pub mod long_term_repair;
mod long_term_repair_catalog;
pub mod mechanism_induction;
pub mod modality;
pub mod nonliteral;
pub mod pragmatic_memory;
pub mod pragmatics;
pub mod professional_document;
pub mod raw_mechanism_induction;
pub mod semantic_roles;
pub mod temporal;

pub use attribution::{
    AttributedPropositionIR, AttributedPropositionPolarityIR, AttributionAnalyzer,
    AttributionAttitudeIR, AttributionEdgeIR, AttributionEvidenceKindIR, AttributionGraphIR,
    AttributionStanceIR, DiscourseActorIR, DiscourseActorKindIR, EpistemicStatusIR,
    ATTRIBUTION_GRAPH_SCHEMA,
};
pub use cognitive::{
    CognitiveApi, CognitiveApiCommandIR, CognitiveApiError, CognitiveApiPayloadIR,
    CognitiveApiResponseIR, ConversationTurnResponseIR, ConversationalOutputIR,
    KnowledgeWorkResponseIR, MechanismInductionResponseIR, NaturalLanguageOutputIR,
    NaturalLanguageRequestIR, NaturalLanguageResponseIR, RawMechanismInductionResponseIR,
    CONVERSATION_TURN_RESPONSE_SCHEMA, NATURAL_LANGUAGE_REQUEST_SCHEMA,
    NATURAL_LANGUAGE_RESPONSE_SCHEMA,
};
pub use compositional_semantics::{
    CandidateDispositionIR, CompositionalAnalysisIR, CompositionalGoalEdgeIR,
    CompositionalGoalGraphIR, CompositionalGoalNodeIR, CompositionalSemanticAnalyzer,
    FrameModalityIR, FrameMoodIR, FramePolarityIR, GoalGraphRelationKindIR,
    InterpretationCandidateIR, PredicateFrameIR, PredicateLexemeError, PredicateLexemeIR,
    PredicateLexiconSnapshotIR, ScopeConstraintIR, ScopeKindIR, COMPOSITIONAL_ANALYSIS_SCHEMA,
    PREDICATE_LEXEME_SCHEMA, PREDICATE_LEXICON_SNAPSHOT_SCHEMA,
};
pub use conditional_guard::{
    ConditionalGuardEvaluationIR, ConditionalGuardIR, ConditionalGuardStoreIR, GuardEvidenceIR,
    GuardEvidencePolarityIR, GuardStatusIR, CONDITIONAL_GUARD_EVALUATION_SCHEMA,
    CONDITIONAL_GUARD_STORE_SCHEMA,
};
pub use conversation::{
    conversational_concept_catalog, validate_conversation_state, ConversationCommitContext,
    ConversationFrontendError, ConversationGoalFrameIR, ConversationInputModalityIR,
    ConversationMemory, ConversationStateIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, ConversationalConceptIR, ConversationalConceptKindIR,
    DiscourseBindingIR, DiscourseBindingKindIR, DiscourseEventIR, DiscourseFunctionIR,
    DiscourseReferentKindIR, DynamicDiscourseReferentIR, DynamicReferentIR,
    NormalizationCandidateIR, NormalizationOperationIR, NormalizationOperationKindIR,
    NormalizedUtteranceIR, ReferenceResolutionIR, UtteranceAlternativeIR, UtteranceNormalizer,
    CONVERSATIONAL_CONCEPT_SCHEMA, CONVERSATION_FRONTEND_SCHEMA, CONVERSATION_STATE_SCHEMA,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
pub use discourse_qa::{
    AnswerClaimKindIR, DiscourseAnswerClaimIR, DiscourseAnswerDispositionIR,
    DiscourseAnswerEvidenceIR, DiscourseAnswerIR, DiscourseQaEngine, DiscourseQueryIR,
    DiscourseQueryKindIR, PresuppositionIR, PresuppositionKindIR, QueryTemporalScopeIR,
    DISCOURSE_ANSWER_SCHEMA, DISCOURSE_QUERY_SCHEMA,
};
pub use document_swarm::{DocumentDeliberationIR, DOCUMENT_DELIBERATION_SCHEMA};
pub use epistemic::{
    proposition_signature, proposition_signature_in_world, BeliefRecordIR, BeliefRecordStatusIR,
    BeliefRevisionIR, BeliefRevisionKindIR, EpistemicLedgerIR, EpistemicObservationIR,
    PropositionSignatureIR, SemanticStateValueIR, TemporalAnchorIR, EPISTEMIC_LEDGER_SCHEMA,
};
pub use generic::DeterministicOffsetCapability;
pub use knowledge_work::{
    analyze_document, analyze_document_in_language, execute_document_work,
    execute_document_work_as, infer_document_design, infer_document_kind, infer_operation,
    BusinessDocumentIR, BusinessDocumentTypeIR, BusinessMetricIR, BusinessSectionIR, CellValueIR,
    ChartIR, ChartPointIR, ChartSeriesIR, ChartTypeIR, DocumentDesignIR, DocumentKindIR,
    DocumentThemeIR, FileOutputReceiptIR, FinancialLineClassIR, FinancialLineItemIR,
    FinancialStatementIR, FinancialStatementTypeIR, FindingKindIR, GuideExampleIR, GuideSectionIR,
    KnowledgeDocumentIR, KnowledgeFindingIR, KnowledgeSourceIR, KnowledgeWorkError,
    KnowledgeWorkOperationIR, KnowledgeWorkProductIR, KnowledgeWorkRequestIR, NumericValueIR,
    OutputDirectiveIR, OutputFormatIR, OutputModeIR, PageSizeIR, PaperClaimIR, PaperIR,
    PaperReferenceIR, PaperSectionIR, PlanProposalIR, PlanTaskIR, SourceTextFormatIR, TableCellIR,
    TableIR, TroubleshootingItemIR, UserGuideIR, BUSINESS_DOCUMENT_SCHEMA, CHART_SCHEMA,
    DOCUMENT_DESIGN_SCHEMA, FINANCIAL_STATEMENT_SCHEMA, KNOWLEDGE_WORK_REQUEST_SCHEMA,
    KNOWLEDGE_WORK_RESPONSE_SCHEMA, PAPER_SCHEMA, PLAN_PROPOSAL_SCHEMA, TABLE_SCHEMA,
    USER_GUIDE_SCHEMA,
};
pub use language::{LanguageAdapter, LanguageAdapterError};
pub use language_knowledge::{
    LanguageCodeIR, LanguageKnowledgeBase, LanguageKnowledgeCategoryIR, LanguageKnowledgeEntryIR,
    LanguageKnowledgeError, LanguageKnowledgeStatisticsIR, LanguageRegisterIR,
    LanguageUnderstandingIR, PragmaticFunctionIR, LANGUAGE_KNOWLEDGE_SCHEMA,
};
pub use lexical_memory::{
    ActivatedSenseIR, GrammaticalRoleIR, LexemeIR, LexemeSnapshotIR, LexemeUsageIR, LexicalMemory,
    LexicalMemoryError, LexicalMemoryStatisticsIR, LexicalOutcomeIR, PartOfSpeechIR,
    SemanticRelationIR, SemanticRelationKindIR, SenseIR, SenseUsageIR, LEXEME_SCHEMA,
    LEXEME_SNAPSHOT_SCHEMA,
};
pub use long_term_repair::{
    process_long_term_repair_plan, ApartmentProfileIR, ApartmentRepairRuleIR, CostInputIR,
    EvidenceExtractionReceiptIR, EvidenceInputIR, EvidenceKindIR, EvidenceStatusIR,
    HouseholdAreaTypeIR, LongTermRepairPlanError, LongTermRepairPlanRequestIR,
    LongTermRepairPlanResponseIR, MonthlyAreaChargeIR, RepairItemDecisionIR, ReportPageIR,
    LONG_TERM_REPAIR_PLAN_REQUEST_SCHEMA, LONG_TERM_REPAIR_PLAN_RESPONSE_SCHEMA,
};
pub use mechanism_induction::{
    MechanismInductionDispositionIR, MechanismInductionEngine, MechanismInductionError,
    MechanismInductionIR, MechanismInductionRequestIR, PropositionLexemeIR,
    StateTransitionObservationIR, TransitionArmIR, MECHANISM_INDUCTION_REQUEST_SCHEMA,
    MECHANISM_INDUCTION_SCHEMA,
};
pub use modality::{
    ConditionalKindIR, ConditionalRelationIR, ModalIllocutionIR, ModalNegationScopeIR,
    ModalOperatorIR, ModalOperatorKindIR, ModalPropositionIR, ModalScopeGraphIR,
    ModalSemanticAnalyzer, ModalWorldIR, MODAL_SCOPE_GRAPH_SCHEMA,
};
pub use nonliteral::{
    NonliteralAnalysisIR, NonliteralAnalyzer, NonliteralExpressionIR, NonliteralKindIR,
    ReadingSelectionIR, NONLITERAL_ANALYSIS_SCHEMA,
};
pub use pragmatic_memory::{
    validate_pragmatic_memory_state, PendingContinuationGateIR, PendingGateStatusIR,
    PragmaticMemory, PragmaticMemoryError, PragmaticMemoryStateIR, PragmaticTaskFrameIR,
    PragmaticTurnSummaryIR, PRAGMATIC_MEMORY_STATE_SCHEMA,
};
pub use pragmatics::{
    ContinuationDecisionGateIR, DecisionBranchActionIR, DiscourseClauseIR, DiscourseClauseRoleIR,
    DiscourseRelationIR, DiscourseRelationKindIR, EvidencePolicyIR, GoalCommitmentIR,
    InferredPragmaticGoalIR, PragmaticContextIR, PragmaticInterpretationIR, PragmaticReasoner,
    PropositionPolarityIR, SpeechActIR, PRAGMATIC_INTERPRETATION_SCHEMA,
};
pub use professional_document::{
    process_professional_document, ConsistencyIssueIR, ConsistencyIssueKindIR,
    ConsistencySeverityIR, DocumentWorkingMemoryIR, EvidenceFactIR, GroundedParagraphIR,
    GroundedSectionDraftIR, LongFormDocumentPlanIR, ParagraphGroundingIR, PriorDocumentSnapshotIR,
    ProfessionalDocumentError, ProfessionalDocumentFileReceiptIR, ProfessionalDocumentKindIR,
    ProfessionalDocumentPageIR, ProfessionalDocumentRequestIR, ProfessionalDocumentResponseIR,
    RevisionDirectiveIR, RevisionRoundIR, SectionMemoryIR, SectionRequirementIR,
    PROFESSIONAL_DOCUMENT_REQUEST_SCHEMA, PROFESSIONAL_DOCUMENT_RESPONSE_SCHEMA,
};
pub use raw_mechanism_induction::{
    AutoPropositionBindingIR, AutoPropositionKindIR, CausalClauseRoleIR, ObservedValueIR,
    RawMechanismInductionEngine, RawMechanismInductionError, RawMechanismInductionIR,
    RawMechanismInductionRequestIR, RawStateTransitionObservationIR,
    RAW_MECHANISM_INDUCTION_REQUEST_SCHEMA, RAW_MECHANISM_INDUCTION_SCHEMA,
};
pub use semantic_roles::{
    EventRelationEdgeIR, EventRelationKindIR, QuantifierKindIR, QuantifierScopeIR, SemanticNodeIR,
    SemanticNodeKindIR, SemanticRoleAnalyzer, SemanticRoleEdgeIR, SemanticRoleGraphIR,
    SemanticRoleKindIR, SEMANTIC_ROLE_GRAPH_SCHEMA,
};
pub use temporal::{
    TemporalAnswerDispositionIR, TemporalAnswerIR, TemporalConflictIR, TemporalEventIR,
    TemporalExpressionIR, TemporalExpressionKindIR, TemporalGraphIR, TemporalQaEngine,
    TemporalQueryIR, TemporalQueryKindIR, TemporalRelationIR, TemporalRelationKindIR,
    TemporalRelationStatusIR, TemporalSemanticAnalyzer, TemporalTurnAnalysisIR,
    TEMPORAL_ANSWER_SCHEMA, TEMPORAL_GRAPH_SCHEMA, TEMPORAL_QUERY_SCHEMA,
};
