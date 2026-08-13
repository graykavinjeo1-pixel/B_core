//! Adapters are deliberately outside `dockable-semantic-core`.

pub mod cognitive;
mod document_design;
pub mod document_swarm;
pub mod generic;
pub mod knowledge_work;
pub mod language;
pub mod language_knowledge;
pub mod lexical_memory;
pub mod long_term_repair;
mod long_term_repair_catalog;
pub mod professional_document;

pub use cognitive::{
    CognitiveApi, CognitiveApiCommandIR, CognitiveApiError, CognitiveApiPayloadIR,
    CognitiveApiResponseIR, KnowledgeWorkResponseIR, NaturalLanguageOutputIR,
    NaturalLanguageRequestIR, NaturalLanguageResponseIR, NATURAL_LANGUAGE_REQUEST_SCHEMA,
    NATURAL_LANGUAGE_RESPONSE_SCHEMA,
};
pub use document_swarm::{DocumentDeliberationIR, DOCUMENT_DELIBERATION_SCHEMA};
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
pub use professional_document::{
    process_professional_document, ConsistencyIssueIR, ConsistencyIssueKindIR,
    ConsistencySeverityIR, DocumentWorkingMemoryIR, EvidenceFactIR, GroundedParagraphIR,
    GroundedSectionDraftIR, LongFormDocumentPlanIR, ParagraphGroundingIR, PriorDocumentSnapshotIR,
    ProfessionalDocumentError, ProfessionalDocumentFileReceiptIR, ProfessionalDocumentKindIR,
    ProfessionalDocumentPageIR, ProfessionalDocumentRequestIR, ProfessionalDocumentResponseIR,
    RevisionDirectiveIR, RevisionRoundIR, SectionMemoryIR, SectionRequirementIR,
    PROFESSIONAL_DOCUMENT_REQUEST_SCHEMA, PROFESSIONAL_DOCUMENT_RESPONSE_SCHEMA,
};
