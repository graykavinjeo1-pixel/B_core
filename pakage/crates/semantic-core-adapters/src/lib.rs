//! Adapters are deliberately outside `dockable-semantic-core`.

pub mod action_state;
pub mod attribution;
pub mod clause_graph;
pub mod cognitive;
pub mod compositional_semantics;
pub mod conditional_guard;
pub mod conversation;
pub mod deferred_commitment;
pub mod definition_grounding;
pub mod deixis_ellipsis;
pub mod discourse_focus;
pub mod discourse_ontology;
pub mod discourse_qa;
pub mod discourse_relations;
mod document_design;
pub mod document_swarm;
pub mod epistemic;
pub mod generative_language;
pub mod generic;
pub mod grammatical_scope;
pub mod grounded_realization;
pub mod interaction_provenance;
pub mod knowledge_work;
pub mod language;
pub mod language_center;
pub mod language_cortex_integration;
pub mod language_knowledge;
pub mod lexical_memory;
pub mod long_term_repair;
mod long_term_repair_catalog;
pub mod mechanism_induction;
pub mod modality;
pub mod native_language_circuit;
pub mod natural_realization;
pub mod nonliteral;
pub mod plan_result_boundary;
pub mod pragmatic_intent;
pub mod pragmatic_memory;
pub mod pragmatics;
pub mod professional_document;
pub mod raw_mechanism_induction;
pub mod reference_resolution_graph;
pub mod semantic_roles;
pub mod six_axis_integration;
pub mod temporal;
pub mod topic_context;
pub mod typed_coreference;
pub mod utterance_intent;

pub use action_state::{
    action_evidence_audit_sha256, action_evidence_receipt_sha256,
    action_language_report_record_sha256, ActionEvidenceAuditIR, ActionEvidenceReceiptIR,
    ActionEvidenceRequestIR, ActionEvidenceStatusIR, ActionExecutionStatusIR,
    ActionLanguageReportIR, ActionLanguageReportRecordIR, ActionPlanSeedIR, ActionPlanStatusIR,
    ActionReportedStatusIR, ActionSetExpressionIR, ActionSetOperatorIR, ActionSetQuantifierIR,
    ActionSetQueryIR, ActionSetTermIR, ActionSetTruthIR, ActionStateAnalysisIR,
    ActionStateAnalyzer, ActionStateLedgerIR, ActionStatePredicateIR, ActionStateRecordIR,
    ACTION_EVIDENCE_AUDIT_SCHEMA, ACTION_EVIDENCE_RECEIPT_SCHEMA, ACTION_EVIDENCE_REQUEST_SCHEMA,
    ACTION_LANGUAGE_REPORT_RECORD_SCHEMA, ACTION_SET_QUERY_SCHEMA, ACTION_STATE_ANALYSIS_SCHEMA,
    ACTION_STATE_LEDGER_SCHEMA,
};
pub use attribution::{
    AttributedPropositionIR, AttributedPropositionPolarityIR, AttributionAnalyzer,
    AttributionAttitudeIR, AttributionEdgeIR, AttributionEvidenceKindIR, AttributionGraphIR,
    AttributionStanceIR, DiscourseActorIR, DiscourseActorKindIR, EpistemicStatusIR,
    ATTRIBUTION_GRAPH_SCHEMA,
};
pub use clause_graph::{
    ClauseFunctionIR, ClauseGraphIR, ClauseNodeIR, ClauseRelationEdgeIR, ClauseRelationKindIR,
    ClauseStructureAnalyzer, CLAUSE_GRAPH_SCHEMA,
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
    conversational_concept_catalog, detect_topic_transition, discourse_program_sha256,
    guard_condition_expression_sha256, validate_conversation_state, ConversationCommitContext,
    ConversationFrontendError, ConversationGoalFrameIR, ConversationInputModalityIR,
    ConversationMemory, ConversationStateIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, ConversationalConceptIR, ConversationalConceptKindIR,
    DialogueDirectiveCandidateIR, DialogueDirectiveIR, DialogueDirectiveKindIR,
    DialogueDirectiveLedgerIR, DialogueDirectiveStatusIR, DiscourseBindingIR,
    DiscourseBindingKindIR, DiscourseEventIR, DiscourseFunctionIR, DiscourseGroupIR,
    DiscourseGroupKindIR, DiscourseGroupUpdateIR, DiscourseGroupUpdateOperationIR,
    DiscourseProgramGuardIR, DiscourseProgramIR, DiscourseProgramStepIR, DiscourseReferentKindIR,
    DiscourseTopicAnchorKindIR, DiscourseTopicIR, DynamicDiscourseReferentIR, DynamicReferentIR,
    GuardConditionExpressionIR, GuardConditionOperatorIR, NormalizationCandidateIR,
    NormalizationOperationIR, NormalizationOperationKindIR, NormalizedUtteranceIR,
    QuestionAnswerDispositionIR, QuestionAnswerResolutionIR, QuestionOptionIR,
    QuestionUnderDiscussionIR, QuestionUnderDiscussionKindIR, ReferenceResolutionIR,
    TopicAnchoredReferenceIR, TopicAnchoredReferentKindIR, TopicAnchoredSelectorKindIR,
    TopicTransitionIR, TopicTransitionKindIR, UtteranceAlternativeIR, UtteranceNormalizer,
    CONVERSATIONAL_CONCEPT_SCHEMA, CONVERSATION_FRONTEND_SCHEMA, CONVERSATION_STATE_SCHEMA,
    CONVERSATION_TURN_REQUEST_SCHEMA, DIALOGUE_DIRECTIVE_LEDGER_SCHEMA,
    DISCOURSE_GROUP_UPDATE_SCHEMA, DISCOURSE_PROGRAM_GUARD_SCHEMA, DISCOURSE_PROGRAM_SCHEMA,
    GUARD_CONDITION_EXPRESSION_SCHEMA, TOPIC_ANCHORED_REFERENCE_SCHEMA, TOPIC_TRANSITION_SCHEMA,
};
pub use deferred_commitment::{
    condition_evidence_receipt_sha256, condition_sha256, normalize_condition,
    ConditionEvidenceDispositionIR, ConditionEvidenceReceiptIR, ConditionEvidenceRequestIR,
    ConditionEvidenceSourceIR, DeferredActionCommitmentIR, DeferredActionIR,
    DeferredCommitmentStatusIR, CONDITION_EVIDENCE_RECEIPT_SCHEMA,
    CONDITION_EVIDENCE_REQUEST_SCHEMA, DEFERRED_ACTION_COMMITMENT_SCHEMA,
};
pub use definition_grounding::{
    DefinitionGrounder, DefinitionGroundingDispositionIR, DefinitionGroundingIR,
    PredicateAliasBindingIR, DEFINITION_GROUNDING_SCHEMA, PREDICATE_ALIAS_BINDING_SCHEMA,
};
pub use deixis_ellipsis::{
    resolve_typed_deixis_or_ellipsis, unresolved_typed_deixis_kind, TypedDeixisEllipsisKindIR,
    TypedDeixisEllipsisResolutionIR, TYPED_DEIXIS_ELLIPSIS_SCHEMA,
};
pub use discourse_focus::{
    DiscourseFocusCandidateIR, DiscourseFocusNodeIR, DiscourseFocusSourceIR, DiscourseFocusStateIR,
    DiscourseFocusStatusIR, DiscourseFocusTransitionIR, DiscourseFocusTransitionKindIR,
    DISCOURSE_FOCUS_STATE_SCHEMA, MAX_DISCOURSE_FOCUS_NODES, MAX_DISCOURSE_FOCUS_TRANSITIONS,
    MAX_DISCOURSE_FOCUS_TURN_DISTANCE,
};
pub use discourse_ontology::{OntologyBindingKind, OntologyReferenceResolution};
pub use discourse_qa::{
    AnswerClaimKindIR, DiscourseAnswerClaimIR, DiscourseAnswerDispositionIR,
    DiscourseAnswerEvidenceIR, DiscourseAnswerIR, DiscourseQaEngine, DiscourseQueryIR,
    DiscourseQueryKindIR, PresuppositionIR, PresuppositionKindIR, QueryTemporalScopeIR,
    DISCOURSE_ANSWER_SCHEMA, DISCOURSE_QUERY_SCHEMA,
};
pub use discourse_relations::{
    DialogueRelationAnswerDispositionIR, DialogueRelationAnswerIR, DialogueRelationEdgeIR,
    DialogueRelationEvidenceIR, DialogueRelationGraphIR, DialogueRelationKindIR,
    DialogueRelationPathIR, DialogueRelationQaEngine, DialogueRelationQueryIR,
    DialogueRelationQueryKindIR, DialogueRelationStatusIR, DIALOGUE_RELATION_ANSWER_SCHEMA,
    DIALOGUE_RELATION_GRAPH_SCHEMA, MAX_DIALOGUE_RELATION_PATHS, MAX_DIALOGUE_RELATION_PATH_HOPS,
};
pub use document_swarm::{DocumentDeliberationIR, DOCUMENT_DELIBERATION_SCHEMA};
pub use epistemic::{
    proposition_signature, proposition_signature_in_world, BeliefRecordIR, BeliefRecordStatusIR,
    BeliefRevisionIR, BeliefRevisionKindIR, EpistemicLedgerIR, EpistemicObservationIR,
    PropositionSignatureIR, SemanticStateValueIR, TemporalAnchorIR, EPISTEMIC_LEDGER_SCHEMA,
};
pub use generative_language::{
    generation_meaning_sha256, generative_language_sha256, DiscourseMoveIR, DiscourseMoveKindIR,
    ExplainableActivationIR, ExpressionMorphologyClassIR, ExpressionNodeIR, ExpressionNodeStore,
    ExpressionPartOfSpeechIR, ExpressionSelectionGraphIR, ExpressionSelectionIR,
    GenerationContextIR, GenerationDiscoursePlanIR, GenerationEmotionIR, GenerationMeaningEdgeIR,
    GenerationMeaningGraphIR, GenerationMeaningNodeIR, GenerationMeaningNodeKindIR,
    GenerationMeaningRelationIR, GenerationSpeechIntentIR, GenerationTenseIR,
    GenerationVerificationIR, GenerativeLanguageCortex, GenerativeLanguageIR,
    GenerativeLanguageRequestIR, MorphologicalRealizationIR, MorphologicalTokenIR,
    SpeechIntentGraphIR, SpeechIntentNodeIR, SyntaxClauseIR, SyntaxConstituentIR,
    SyntaxConstituentRoleIR, SyntaxPlanIR, GENERATION_MEANING_SCHEMA, GENERATIVE_LANGUAGE_SCHEMA,
};
pub use generic::DeterministicOffsetCapability;
pub use grammatical_scope::{
    grammatical_scope_graph_sha256, GrammaticalScopeAnalyzer, GrammaticalScopeEdgeIR,
    GrammaticalScopeEdgeKindIR, GrammaticalScopeGraphIR, GrammaticalScopeNodeIR,
    GrammaticalScopeNodeKindIR, GRAMMATICAL_SCOPE_GRAPH_SCHEMA,
};
pub use grounded_realization::{
    ClaimEpistemicStatusIR, ClaimSupportStatusIR, EvidenceGroundedRealizationIR, GroundedClaimIR,
    GroundedClaimKindIR, EVIDENCE_GROUNDED_REALIZATION_SCHEMA,
};
pub use interaction_provenance::{
    build_interaction_provenance, interaction_provenance_edge_sha256,
    interaction_provenance_graph_sha256, interaction_provenance_node_sha256,
    InteractionProvenanceEdgeIR, InteractionProvenanceGraphIR, InteractionProvenanceNodeIR,
    InteractionProvenanceNodeKindIR, InteractionProvenanceRelationIR, InteractionProvenanceSources,
    INTERACTION_PROVENANCE_GRAPH_SCHEMA,
};
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
pub use language_center::{
    compositional_analysis_sha256, language_center_goal_projection_sha256,
    language_center_graph_sha256, language_center_semantic_sha256, LanguageCenterArgumentIR,
    LanguageCenterConflictIR, LanguageCenterContributionIR, LanguageCenterEventIR,
    LanguageCenterGoalDecisionIR, LanguageCenterGoalDecisionSourceIR, LanguageCenterGoalEffectIR,
    LanguageCenterGoalProjectionIR, LanguageCenterIR, LanguageCenterPipeline,
    LanguageCenterProjectionIR, LanguageCenterRelationIR, LanguageCenterSourceIR,
    LanguageCenterSources, LANGUAGE_CENTER_GOAL_PROJECTION_SCHEMA, LANGUAGE_CENTER_SCHEMA,
};
pub use language_cortex_integration::{
    build_language_cortex_response_integration, language_cortex_response_integration_sha256,
    LanguageCortexResponseIntegrationIR, LanguageCortexResponseSources,
    LANGUAGE_CORTEX_RESPONSE_INTEGRATION_SCHEMA,
};
pub use language_knowledge::{
    LanguageCodeIR, LanguageDialogueDirectiveAnalysisIR, LanguageDialogueDirectiveAtomIR,
    LanguageDialogueDirectiveAxisIR, LanguageDialogueDirectiveFrameIR,
    LanguageDialogueDirectiveValueIR, LanguageKnowledgeBase, LanguageKnowledgeCategoryIR,
    LanguageKnowledgeEntryIR, LanguageKnowledgeError, LanguageKnowledgeStatisticsIR,
    LanguageRegisterIR, LanguageUnderstandingIR, PragmaticFunctionIR,
    LANGUAGE_DIALOGUE_DIRECTIVE_ANALYSIS_SCHEMA, LANGUAGE_KNOWLEDGE_SCHEMA,
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
pub use native_language_circuit::{
    NativeContextEntityIR, NativeContextGoalIR, NativeContextReferentIR, NativeDialogueContextIR,
    NativeDiscourseRelationIR, NativeEntityIR, NativeEventIR, NativeEventScopeIR, NativeGoalIR,
    NativeLanguageCircuit, NativeReferenceBindingIR, NativeReferenceKindIR, NativeRelationEdgeIR,
    NativeResponseGoalIR, NativeResponseModeIR, NativeTurnIR, NATIVE_LANGUAGE_CIRCUIT_SCHEMA,
};
pub use natural_realization::{
    arbitrate_natural_response, natural_realization_coverage_sha256, natural_realization_sha256,
    natural_response_arbitration_sha256, NaturalRealizationCoverageIR, NaturalRealizationIR,
    NaturalRealizationObligationIR, NaturalRealizationObligationKindIR, NaturalRealizationPathIR,
    NaturalResponseActIR, NaturalResponseArbitrationIR, NaturalResponseCandidateIR,
    NaturalResponseFormatIR, NaturalResponseMoveIR, NaturalResponseMoveRoleIR,
    NaturalResponsePlanIR, NaturalResponseSourceIR, NaturalSentenceFunctionIR, NaturalSentenceIR,
    NATURAL_REALIZATION_COVERAGE_SCHEMA, NATURAL_REALIZATION_SCHEMA,
};
pub use nonliteral::{
    NonliteralAnalysisIR, NonliteralAnalyzer, NonliteralExpressionIR, NonliteralKindIR,
    ReadingSelectionIR, NONLITERAL_ANALYSIS_SCHEMA,
};
pub use plan_result_boundary::{
    boundary_sha256 as plan_result_boundary_sha256, build_plan_result_boundary,
    classify_plan_result_query_focus, ActionLifecycleSnapshotIR, PlanResultBoundaryIR,
    PlanResultQueryFocusIR, ResultAvailabilityIR, ACTION_LIFECYCLE_SNAPSHOT_SCHEMA,
    PLAN_RESULT_BOUNDARY_SCHEMA,
};
pub use pragmatic_intent::{
    PragmaticGoalProjectionIR, PragmaticIntentAnalyzer, PragmaticIntentGraphIR,
    PragmaticIntentInferenceIR, PragmaticIntentKindIR, PRAGMATIC_INTENT_GRAPH_SCHEMA,
};
pub use pragmatic_memory::{
    validate_pragmatic_memory_state, PendingContinuationGateIR, PendingGateStatusIR,
    PragmaticMemory, PragmaticMemoryError, PragmaticMemoryStateIR, PragmaticTaskFrameIR,
    PragmaticTurnSummaryIR, PRAGMATIC_MEMORY_STATE_SCHEMA,
};
pub use pragmatics::{
    ActiveGoalContextIR, CommitmentActivationIR, ContinuationDecisionGateIR,
    DecisionBranchActionIR, DialogueParticipantIR, DiscourseClauseIR, DiscourseClauseRoleIR,
    DiscourseRelationIR, DiscourseRelationKindIR, EvidencePolicyIR, GoalCommitmentIR,
    GoalWithdrawalIR, GoalWithdrawalScopeIR, IllocutionaryCommitmentGraphIR,
    IllocutionaryCommitmentIR, IllocutionaryForceIR, InferredPragmaticGoalIR, OutcomeClaimPolicyIR,
    PendingDeferredContextIR, PragmaticContextIR, PragmaticInterpretationIR, PragmaticReasoner,
    PropositionPolarityIR, RequiredOutcomeEvidenceIR, SpeechActIR, UserFeedbackIR,
    UserFeedbackKindIR, PRAGMATIC_INTERPRETATION_SCHEMA,
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
pub use reference_resolution_graph::{
    build_reference_resolution_graph, scan_reference_mentions, ReferenceAntecedentCandidateIR,
    ReferenceCandidateEdgeIR, ReferenceMentionKindIR, ReferenceMentionNodeIR,
    ReferenceResolutionGraphIR, ReferenceSelectionHint, MAX_REFERENCE_CANDIDATES,
    MAX_REFERENCE_EDGES, MAX_REFERENCE_MENTIONS, REFERENCE_RESOLUTION_GRAPH_SCHEMA,
};
pub use semantic_roles::{
    EventRelationEdgeIR, EventRelationKindIR, QuantifierKindIR, QuantifierScopeIR,
    RelativeClauseAttachmentIR, SemanticNodeIR, SemanticNodeKindIR, SemanticRoleAnalyzer,
    SemanticRoleEdgeIR, SemanticRoleGraphIR, SemanticRoleKindIR, SharedArgumentBindingIR,
    SharedArgumentDirectionIR, SEMANTIC_ROLE_GRAPH_SCHEMA,
};
pub use six_axis_integration::{
    build_six_axis_integration, language_cortex_package_boundary,
    language_cortex_package_boundary_sha256, six_axis_integration_sha256, CrossAxisInvariantIR,
    CrossAxisInvariantKindIR, LanguageAxisEvidenceIR, LanguageAxisIR, LanguageAxisStatusIR,
    LanguageCortexPackageBoundaryIR, SixAxisIntegrationIR, SixAxisIntegrationSources,
    LANGUAGE_CORTEX_PACKAGE_BOUNDARY_SCHEMA, SIX_AXIS_INTEGRATION_SCHEMA,
};
pub use temporal::{
    TemporalAnswerDispositionIR, TemporalAnswerIR, TemporalConflictIR, TemporalEventIR,
    TemporalExpressionIR, TemporalExpressionKindIR, TemporalGraphIR, TemporalQaEngine,
    TemporalQueryIR, TemporalQueryKindIR, TemporalRelationIR, TemporalRelationKindIR,
    TemporalRelationStatusIR, TemporalSemanticAnalyzer, TemporalTurnAnalysisIR,
    TEMPORAL_ANSWER_SCHEMA, TEMPORAL_GRAPH_SCHEMA, TEMPORAL_QUERY_SCHEMA,
};
pub use topic_context::{
    topic_context_graph_sha256, topic_context_sha256, topic_context_transition_sha256,
    TopicContextGraphIR, TopicContextIR, TopicContextStatusIR, TopicContextTransitionIR,
    TopicContextTransitionKindIR, MAX_TOPIC_CONTEXTS, MAX_TOPIC_CONTEXT_REFERENTS,
    MAX_TOPIC_CONTEXT_TRANSITIONS, TOPIC_CONTEXT_GRAPH_SCHEMA,
};
pub use typed_coreference::{
    TypedCoreferenceBindingKind, TypedCoreferenceResolution, TypedEntityKindIR,
    TypedEntityReferentIR, TypedMentionRoleIR, MAX_TYPED_ENTITY_REFERENTS,
    MAX_TYPED_REFERENCE_TURN_DISTANCE,
};
pub use utterance_intent::{
    CommunicativeIntentIR, ExpectedResponseKindIR, UtteranceIntentAnalyzer,
    UtteranceIntentCandidateIR, UtteranceIntentGraphIR, UtteranceIntentSignalIR,
    UtteranceSignalKindIR, UtteranceSurfaceFormIR, MAX_UTTERANCE_INTENT_CANDIDATES,
    MAX_UTTERANCE_INTENT_SIGNALS, UTTERANCE_INTENT_GRAPH_SCHEMA,
};
