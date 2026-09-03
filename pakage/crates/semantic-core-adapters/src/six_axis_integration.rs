use std::collections::BTreeSet;

use dockable_semantic_core::PlanIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::action_state::{
    ActionExecutionStatusIR, ActionStateAnalysisIR, ActionStateLedgerIR,
    ACTION_STATE_ANALYSIS_SCHEMA,
};
use crate::compositional_semantics::{CompositionalAnalysisIR, COMPOSITIONAL_ANALYSIS_SCHEMA};
use crate::conversation::{
    ConversationStateIR, DiscourseBindingKindIR, ReferenceResolutionIR, CONVERSATION_STATE_SCHEMA,
};
use crate::grounded_realization::EvidenceGroundedRealizationIR;
use crate::interaction_provenance::{
    InteractionProvenanceGraphIR, InteractionProvenanceNodeKindIR,
};
use crate::natural_realization::{NaturalRealizationIR, NaturalResponseActIR};
use crate::plan_result_boundary::{PlanResultBoundaryIR, PLAN_RESULT_BOUNDARY_SCHEMA};
use crate::pragmatics::{PragmaticInterpretationIR, PRAGMATIC_INTERPRETATION_SCHEMA};

pub const SIX_AXIS_INTEGRATION_SCHEMA: &str = "B_CORE_SIX_AXIS_INTEGRATION_IR_2";
pub const LANGUAGE_CORTEX_PACKAGE_BOUNDARY_SCHEMA: &str =
    "B_CORE_LANGUAGE_CORTEX_PACKAGE_BOUNDARY_IR_1";
const MAX_AXIS_EVIDENCE_REFS: usize = 32;
const MAX_INTEGRATION_VIOLATIONS: usize = 32;
const LANGUAGE_ADAPTER_MANIFEST: &str = include_str!("../Cargo.toml");
const SEMANTIC_CORE_MANIFEST: &str = include_str!("../../dockable-semantic-core/Cargo.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageAxisIR {
    GrammaticalComposition,
    DiscourseTopicState,
    DeixisEllipsis,
    PragmaticIntent,
    PlanResultBoundary,
    EvidenceGroundedRealization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CrossAxisLinkKindIR {
    GrammaticalCompositionToPragmaticIntent,
    DiscourseToReferenceResolution,
    ReferenceToPragmaticIntent,
    PragmaticIntentToPlan,
    PlanResultToNaturalRealization,
    DiscourseToNaturalRealization,
    ReferenceToNaturalRealization,
    NaturalToGroundedRealization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossAxisLinkIR {
    pub kind: CrossAxisLinkKindIR,
    pub source_axis: LanguageAxisIR,
    pub target_axis: LanguageAxisIR,
    pub active: bool,
    pub satisfied: bool,
    pub evidence_refs: Vec<String>,
}

impl CrossAxisLinkIR {
    fn validate(&self) -> bool {
        !self.evidence_refs.is_empty()
            && self.evidence_refs.len() <= MAX_AXIS_EVIDENCE_REFS
            && self
                .evidence_refs
                .iter()
                .all(|reference| valid_text(reference))
            && (!self.active || self.satisfied)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageAxisStatusIR {
    Pass,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageAxisEvidenceIR {
    pub axis: LanguageAxisIR,
    pub status: LanguageAxisStatusIR,
    pub component_schema: String,
    pub component_sha256: String,
    pub evidence_refs: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
}

impl LanguageAxisEvidenceIR {
    fn validate(&self) -> bool {
        !self.component_schema.trim().is_empty()
            && valid_digest(&self.component_sha256)
            && !self.evidence_refs.is_empty()
            && self.evidence_refs.len() <= MAX_AXIS_EVIDENCE_REFS
            && self
                .evidence_refs
                .iter()
                .all(|reference| valid_text(reference))
            && !self.semantic_authority
            && !self.external_action_executed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CrossAxisInvariantKindIR {
    TurnStateAligned,
    AmbiguityFailsClosed,
    GoalPlanProvenanceComplete,
    LanguageReportCannotVerifyResult,
    RealizationClaimsHaveTypedSources,
    OutputMatchesGroundedRealization,
    NaturalRealizationMatchesGroundedRealization,
    ActiveCrossAxisLinksCoherent,
    LanguageSemanticAuthorityAbsent,
    PackageDependencyPointsTowardCore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossAxisInvariantIR {
    pub kind: CrossAxisInvariantKindIR,
    pub satisfied: bool,
    pub evidence_refs: Vec<String>,
}

impl CrossAxisInvariantIR {
    fn validate(&self) -> bool {
        !self.evidence_refs.is_empty()
            && self.evidence_refs.len() <= MAX_AXIS_EVIDENCE_REFS
            && self
                .evidence_refs
                .iter()
                .all(|reference| valid_text(reference))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCortexPackageBoundaryIR {
    pub schema: String,
    pub semantic_core_crate: String,
    pub language_adapter_crate: String,
    pub dependency_direction: String,
    pub semantic_core_manifest_sha256: String,
    pub language_adapter_manifest_sha256: String,
    pub raw_language_reaches_core: bool,
    pub adapter_owns_semantic_state: bool,
    pub semantic_authority: bool,
    pub external_action_execution_authority: bool,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub network_calls: usize,
    pub recursive_source_mutations: usize,
    pub valid: bool,
    pub boundary_sha256: String,
}

impl LanguageCortexPackageBoundaryIR {
    pub fn validate(&self) -> bool {
        self.schema == LANGUAGE_CORTEX_PACKAGE_BOUNDARY_SCHEMA
            && self.semantic_core_crate == "dockable-semantic-core"
            && self.language_adapter_crate == "semantic-core-adapters"
            && self.dependency_direction == "LANGUAGE_ADAPTER_TO_SEMANTIC_CORE_ONLY"
            && self.semantic_core_manifest_sha256 == text_sha256(SEMANTIC_CORE_MANIFEST)
            && self.language_adapter_manifest_sha256 == text_sha256(LANGUAGE_ADAPTER_MANIFEST)
            && manifests_have_one_way_dependency()
            && !self.raw_language_reaches_core
            && !self.adapter_owns_semantic_state
            && !self.semantic_authority
            && !self.external_action_execution_authority
            && self.external_llm_calls == 0
            && self.local_teacher_calls == 0
            && self.network_calls == 0
            && self.recursive_source_mutations == 0
            && self.valid
            && valid_digest(&self.boundary_sha256)
            && self.boundary_sha256 == language_cortex_package_boundary_sha256(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SixAxisIntegrationIR {
    pub schema: String,
    pub request_id: String,
    pub turn_index: u64,
    pub axes: Vec<LanguageAxisEvidenceIR>,
    pub cross_axis_links: Vec<CrossAxisLinkIR>,
    pub cross_axis_invariants: Vec<CrossAxisInvariantIR>,
    pub violations: Vec<String>,
    pub package_boundary: LanguageCortexPackageBoundaryIR,
    pub complete: bool,
    pub semantic_authority: bool,
    pub language_can_execute: bool,
    pub natural_realization_sha256: String,
    pub integration_sha256: String,
}

impl SixAxisIntegrationIR {
    pub fn validate(&self) -> bool {
        let axes = self
            .axes
            .iter()
            .map(|axis| axis.axis)
            .collect::<BTreeSet<_>>();
        let invariants = self
            .cross_axis_invariants
            .iter()
            .map(|invariant| invariant.kind)
            .collect::<BTreeSet<_>>();
        let links = self
            .cross_axis_links
            .iter()
            .map(|link| link.kind)
            .collect::<BTreeSet<_>>();
        self.schema == SIX_AXIS_INTEGRATION_SCHEMA
            && valid_text(&self.request_id)
            && self.turn_index > 0
            && self.axes.len() == 6
            && axes.len() == self.axes.len()
            && axes == required_axes()
            && self.axes.iter().all(LanguageAxisEvidenceIR::validate)
            && self.cross_axis_links.len() == 8
            && links.len() == self.cross_axis_links.len()
            && links == required_links()
            && self.cross_axis_links.iter().all(CrossAxisLinkIR::validate)
            && self.cross_axis_invariants.len() == 10
            && invariants.len() == self.cross_axis_invariants.len()
            && self
                .cross_axis_invariants
                .iter()
                .all(CrossAxisInvariantIR::validate)
            && self.violations.len() <= MAX_INTEGRATION_VIOLATIONS
            && self
                .violations
                .iter()
                .all(|violation| valid_text(violation))
            && self.package_boundary.validate()
            && self.complete
                == (self.violations.is_empty()
                    && self
                        .axes
                        .iter()
                        .all(|axis| axis.status == LanguageAxisStatusIR::Pass)
                    && self.cross_axis_links.iter().all(|link| link.satisfied)
                    && self
                        .cross_axis_invariants
                        .iter()
                        .all(|invariant| invariant.satisfied))
            && !self.semantic_authority
            && !self.language_can_execute
            && valid_digest(&self.natural_realization_sha256)
            && valid_digest(&self.integration_sha256)
            && self.integration_sha256 == six_axis_integration_sha256(self)
    }

    pub fn validate_against(&self, sources: SixAxisIntegrationSources<'_>) -> bool {
        self.validate() && self == &build_six_axis_integration(sources)
    }
}

#[derive(Clone, Copy)]
pub struct SixAxisIntegrationSources<'a> {
    pub request_id: &'a str,
    pub turn_index: u64,
    pub pragmatic_interpretation: &'a PragmaticInterpretationIR,
    pub conversation_state: &'a ConversationStateIR,
    pub reference_resolution: &'a ReferenceResolutionIR,
    pub action_state_analysis: &'a ActionStateAnalysisIR,
    pub plan_result_boundary: &'a PlanResultBoundaryIR,
    pub grounded_plan: Option<&'a PlanIR>,
    pub natural_realization: &'a NaturalRealizationIR,
    pub grounded_realization: &'a EvidenceGroundedRealizationIR,
    pub interaction_provenance: &'a InteractionProvenanceGraphIR,
    pub realized_output: &'a str,
}

pub fn language_cortex_package_boundary() -> LanguageCortexPackageBoundaryIR {
    let mut boundary = LanguageCortexPackageBoundaryIR {
        schema: LANGUAGE_CORTEX_PACKAGE_BOUNDARY_SCHEMA.to_string(),
        semantic_core_crate: "dockable-semantic-core".to_string(),
        language_adapter_crate: "semantic-core-adapters".to_string(),
        dependency_direction: "LANGUAGE_ADAPTER_TO_SEMANTIC_CORE_ONLY".to_string(),
        semantic_core_manifest_sha256: text_sha256(SEMANTIC_CORE_MANIFEST),
        language_adapter_manifest_sha256: text_sha256(LANGUAGE_ADAPTER_MANIFEST),
        raw_language_reaches_core: false,
        adapter_owns_semantic_state: false,
        semantic_authority: false,
        external_action_execution_authority: false,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
        valid: true,
        boundary_sha256: String::new(),
    };
    boundary.boundary_sha256 = language_cortex_package_boundary_sha256(&boundary);
    boundary
}

pub fn build_six_axis_integration(sources: SixAxisIntegrationSources<'_>) -> SixAxisIntegrationIR {
    let composition = &sources.pragmatic_interpretation.compositional_analysis;
    let intent = &sources.pragmatic_interpretation.pragmatic_intent_graph;
    let ledger = &sources.conversation_state.action_state_ledger;
    let grammatical_ok = sources.pragmatic_interpretation.schema == PRAGMATIC_INTERPRETATION_SCHEMA
        && composition.schema == COMPOSITIONAL_ANALYSIS_SCHEMA
        && composition.grammatical_scope_graph.validate()
        && composition.structural_coverage_millis <= 1_000;
    let discourse_ok = sources.conversation_state.schema == CONVERSATION_STATE_SCHEMA
        && sources.conversation_state.completed_turns == sources.turn_index
        && valid_digest(&sources.conversation_state.state_sha256);
    let reference_ok = references_are_consistent(sources.reference_resolution);
    let intent_ok = !intent.semantic_authority
        && !intent.external_action_execution_authorized
        && sources.pragmatic_interpretation.language_center.validate()
        && !sources
            .pragmatic_interpretation
            .language_center
            .semantic_authority
        && !sources
            .pragmatic_interpretation
            .language_center
            .language_can_execute
        && sources
            .pragmatic_interpretation
            .language_center_goal_projection
            .as_ref()
            .is_some_and(|projection| {
                projection.validate_against(
                    &sources.pragmatic_interpretation.language_center,
                    composition,
                )
            })
        && intent
            .composition
            .as_ref()
            .is_none_or(|composition| composition.validate());
    let provenance_ok = sources
        .interaction_provenance
        .validate_against(sources.grounded_realization, ledger);
    let plan_result_ok = sources.action_state_analysis.schema == ACTION_STATE_ANALYSIS_SCHEMA
        && !sources.action_state_analysis.semantic_authority
        && !sources.action_state_analysis.external_action_executed
        && ledger.validate(sources.turn_index)
        && sources.plan_result_boundary.validate()
        && provenance_ok
        && reports_do_not_establish_results(ledger, sources.interaction_provenance);
    let realization_ok = sources.grounded_realization.validate()
        && sources.natural_realization.validate()
        && sources.natural_realization.realized_text == sources.realized_output
        && sources.grounded_realization.realized_text == sources.realized_output
        && sources.natural_realization.unsupported_claims
            == sources.grounded_realization.unsupported_claims
        && provenance_ok;

    let axes = vec![
        axis(
            LanguageAxisIR::GrammaticalComposition,
            grammatical_ok,
            &composition.schema,
            composition,
            composition_evidence(composition),
        ),
        axis(
            LanguageAxisIR::DiscourseTopicState,
            discourse_ok,
            &sources.conversation_state.schema,
            sources.conversation_state,
            discourse_evidence(sources.conversation_state),
        ),
        axis(
            LanguageAxisIR::DeixisEllipsis,
            reference_ok,
            "B_CORE_REFERENCE_RESOLUTION_IR_1",
            sources.reference_resolution,
            reference_evidence(sources.reference_resolution),
        ),
        axis(
            LanguageAxisIR::PragmaticIntent,
            intent_ok,
            &intent.schema,
            intent,
            pragmatic_evidence(sources.pragmatic_interpretation),
        ),
        axis(
            LanguageAxisIR::PlanResultBoundary,
            plan_result_ok,
            PLAN_RESULT_BOUNDARY_SCHEMA,
            sources.plan_result_boundary,
            plan_result_evidence(
                sources.plan_result_boundary,
                sources.grounded_plan,
                sources.interaction_provenance,
            ),
        ),
        axis(
            LanguageAxisIR::EvidenceGroundedRealization,
            realization_ok,
            &sources.grounded_realization.schema,
            sources.grounded_realization,
            vec![
                sources.natural_realization.realization_sha256.clone(),
                sources.grounded_realization.realization_sha256.clone(),
                sources.interaction_provenance.graph_sha256.clone(),
            ],
        ),
    ];

    let composition_selected = composition.selected_candidate_id.is_some()
        || !composition.selected_candidate_ids.is_empty()
        || composition
            .goal_graph
            .as_ref()
            .is_some_and(|graph| !graph.nodes.is_empty());
    let intent_selected = sources.pragmatic_interpretation.inferred_goal.is_some()
        || intent.primary.is_some()
        || intent
            .composition
            .as_ref()
            .is_some_and(|composition| !composition.selected_node_ids.is_empty());
    let contextual_intent_evidence =
        contextual_continuation_resolution_evidence(sources.natural_realization);
    let contextual_intent_selected = !contextual_intent_evidence.is_empty();
    let reference_active = sources.reference_resolution.resolved_reference_count > 0
        || !sources.reference_resolution.used_referent_ids.is_empty()
        || !sources.reference_resolution.discourse_bindings.is_empty()
        || !sources
            .reference_resolution
            .ambiguous_reference_surfaces
            .is_empty()
        || sources
            .reference_resolution
            .topic_anchored_resolution
            .as_ref()
            .is_some_and(|resolution| resolution.applied);
    let reference_unambiguous = reference_active
        && sources
            .reference_resolution
            .ambiguous_reference_surfaces
            .is_empty();
    let topic_reference_active = sources
        .reference_resolution
        .topic_anchored_resolution
        .as_ref()
        .is_some_and(|resolution| resolution.applied);
    let natural_has_plan = sources.grounded_plan.is_some_and(|plan| {
        natural_source_ref(sources.natural_realization, "PLAN", &plan.plan_sha256)
    });
    let natural_has_boundary = natural_source_ref(
        sources.natural_realization,
        "PLAN_RESULT_BOUNDARY",
        &sources.plan_result_boundary.boundary_sha256,
    );
    let natural_has_reference =
        sources
            .reference_resolution
            .used_referent_ids
            .iter()
            .any(|referent_id| {
                natural_source_ref(sources.natural_realization, "REFERENT", referent_id)
            })
            || sources
                .reference_resolution
                .topic_anchored_resolution
                .as_ref()
                .is_some_and(|resolution| {
                    natural_source_ref(
                        sources.natural_realization,
                        "TOPIC_REFERENCE",
                        &resolution.resolution_sha256,
                    )
                })
            || sources
                .reference_resolution
                .discourse_bindings
                .iter()
                .filter_map(|binding| binding.inherited_goal_id.as_deref())
                .any(|goal_id| {
                    natural_source_ref(sources.natural_realization, "REFERENCE_GOAL", goal_id)
                })
            || natural_source_kind(sources.natural_realization, "REFERENCE_BINDING")
            || natural_source_kind(sources.natural_realization, "AMBIGUOUS_REFERENCE");
    let natural_has_topic = sources
        .reference_resolution
        .topic_anchored_resolution
        .as_ref()
        .is_some_and(|resolution| {
            natural_source_ref(
                sources.natural_realization,
                "TOPIC_REFERENCE",
                &resolution.resolution_sha256,
            )
        })
        || matches!(
            sources.natural_realization.response_act,
            NaturalResponseActIR::TopicTransition
                | NaturalResponseActIR::DiscourseAnswer
                | NaturalResponseActIR::DialogueRelationAnswer
                | NaturalResponseActIR::TemporalAnswer
        );
    let plan_result_natural_active = matches!(
        sources.natural_realization.response_act,
        NaturalResponseActIR::PlanResultStatus | NaturalResponseActIR::ResultAbsence
    );
    let natural_grounded_aligned = sources.natural_realization.validate()
        && sources.grounded_realization.validate()
        && sources.natural_realization.realized_text == sources.grounded_realization.realized_text
        && sources.natural_realization.unsupported_claims
            == sources.grounded_realization.unsupported_claims;
    let mut composition_intent_evidence = vec![content_sha256(composition), content_sha256(intent)];
    composition_intent_evidence.extend(contextual_intent_evidence);
    let cross_axis_links = vec![
        cross_axis_link(
            CrossAxisLinkKindIR::GrammaticalCompositionToPragmaticIntent,
            LanguageAxisIR::GrammaticalComposition,
            LanguageAxisIR::PragmaticIntent,
            composition_selected,
            !composition_selected || intent_selected || contextual_intent_selected,
            composition_intent_evidence,
        ),
        cross_axis_link(
            CrossAxisLinkKindIR::DiscourseToReferenceResolution,
            LanguageAxisIR::DiscourseTopicState,
            LanguageAxisIR::DeixisEllipsis,
            reference_active,
            !reference_active || reference_ok,
            vec![
                sources.conversation_state.state_sha256.clone(),
                content_sha256(sources.reference_resolution),
            ],
        ),
        cross_axis_link(
            CrossAxisLinkKindIR::ReferenceToPragmaticIntent,
            LanguageAxisIR::DeixisEllipsis,
            LanguageAxisIR::PragmaticIntent,
            reference_unambiguous && (intent_selected || sources.grounded_plan.is_some()),
            !(reference_unambiguous && (intent_selected || sources.grounded_plan.is_some()))
                || intent_ok,
            vec![
                content_sha256(sources.reference_resolution),
                content_sha256(intent),
            ],
        ),
        cross_axis_link(
            CrossAxisLinkKindIR::PragmaticIntentToPlan,
            LanguageAxisIR::PragmaticIntent,
            LanguageAxisIR::PlanResultBoundary,
            sources.grounded_plan.is_some(),
            sources.grounded_plan.is_none()
                || (intent_ok
                    && has_goal_plan_edge(sources.interaction_provenance)
                    && natural_has_plan),
            vec![
                content_sha256(intent),
                sources.interaction_provenance.graph_sha256.clone(),
            ],
        ),
        cross_axis_link(
            CrossAxisLinkKindIR::PlanResultToNaturalRealization,
            LanguageAxisIR::PlanResultBoundary,
            LanguageAxisIR::EvidenceGroundedRealization,
            plan_result_natural_active,
            !plan_result_natural_active || natural_has_boundary,
            vec![
                sources.plan_result_boundary.boundary_sha256.clone(),
                sources.natural_realization.realization_sha256.clone(),
            ],
        ),
        cross_axis_link(
            CrossAxisLinkKindIR::DiscourseToNaturalRealization,
            LanguageAxisIR::DiscourseTopicState,
            LanguageAxisIR::EvidenceGroundedRealization,
            topic_reference_active
                || matches!(
                    sources.natural_realization.response_act,
                    NaturalResponseActIR::TopicTransition
                        | NaturalResponseActIR::DiscourseAnswer
                        | NaturalResponseActIR::DialogueRelationAnswer
                        | NaturalResponseActIR::TemporalAnswer
                ),
            !(topic_reference_active
                || matches!(
                    sources.natural_realization.response_act,
                    NaturalResponseActIR::TopicTransition
                        | NaturalResponseActIR::DiscourseAnswer
                        | NaturalResponseActIR::DialogueRelationAnswer
                        | NaturalResponseActIR::TemporalAnswer
                ))
                || natural_has_topic,
            vec![
                sources.conversation_state.state_sha256.clone(),
                sources.natural_realization.realization_sha256.clone(),
            ],
        ),
        cross_axis_link(
            CrossAxisLinkKindIR::ReferenceToNaturalRealization,
            LanguageAxisIR::DeixisEllipsis,
            LanguageAxisIR::EvidenceGroundedRealization,
            reference_active,
            !reference_active || natural_has_reference,
            vec![
                content_sha256(sources.reference_resolution),
                sources.natural_realization.realization_sha256.clone(),
            ],
        ),
        cross_axis_link(
            CrossAxisLinkKindIR::NaturalToGroundedRealization,
            LanguageAxisIR::EvidenceGroundedRealization,
            LanguageAxisIR::EvidenceGroundedRealization,
            true,
            natural_grounded_aligned,
            vec![
                sources.natural_realization.realization_sha256.clone(),
                sources.grounded_realization.realization_sha256.clone(),
            ],
        ),
    ];

    let ambiguity_present = composition.clarification_required
        || !sources
            .reference_resolution
            .ambiguous_reference_surfaces
            .is_empty()
        || !sources
            .pragmatic_interpretation
            .unresolved_bindings
            .is_empty()
        || !intent.unresolved_ambiguities.is_empty();
    let ambiguity_fails_closed = !ambiguity_present
        || (sources.grounded_plan.is_none()
            && !sources.action_state_analysis.external_action_executed);
    let goal_plan_complete =
        sources.grounded_plan.is_none() || has_goal_plan_edge(sources.interaction_provenance);
    let language_non_authoritative = axes
        .iter()
        .all(|axis| !axis.semantic_authority && !axis.external_action_executed)
        && !sources.interaction_provenance.semantic_authority
        && !sources
            .interaction_provenance
            .language_can_advance_execution;
    let package_boundary = language_cortex_package_boundary();
    let cross_axis_invariants = vec![
        invariant(
            CrossAxisInvariantKindIR::TurnStateAligned,
            discourse_ok,
            vec![sources.conversation_state.state_sha256.clone()],
        ),
        invariant(
            CrossAxisInvariantKindIR::AmbiguityFailsClosed,
            ambiguity_fails_closed,
            vec![format!("AMBIGUITY_PRESENT:{ambiguity_present}")],
        ),
        invariant(
            CrossAxisInvariantKindIR::GoalPlanProvenanceComplete,
            goal_plan_complete,
            vec![sources.interaction_provenance.graph_sha256.clone()],
        ),
        invariant(
            CrossAxisInvariantKindIR::LanguageReportCannotVerifyResult,
            reports_do_not_establish_results(ledger, sources.interaction_provenance),
            vec![format!(
                "REPORTS:{}:AUDITS:{}",
                ledger.language_report_history.len(),
                ledger.evidence_audit_history.len()
            )],
        ),
        invariant(
            CrossAxisInvariantKindIR::RealizationClaimsHaveTypedSources,
            provenance_ok,
            vec![sources.interaction_provenance.graph_sha256.clone()],
        ),
        invariant(
            CrossAxisInvariantKindIR::OutputMatchesGroundedRealization,
            sources.grounded_realization.realized_text == sources.realized_output,
            vec![sources.grounded_realization.realization_sha256.clone()],
        ),
        invariant(
            CrossAxisInvariantKindIR::NaturalRealizationMatchesGroundedRealization,
            natural_grounded_aligned,
            vec![
                sources.natural_realization.realization_sha256.clone(),
                sources.grounded_realization.realization_sha256.clone(),
            ],
        ),
        invariant(
            CrossAxisInvariantKindIR::ActiveCrossAxisLinksCoherent,
            cross_axis_links.iter().all(|link| link.satisfied),
            cross_axis_links
                .iter()
                .map(|link| format!("{:?}:{}", link.kind, link.satisfied))
                .collect(),
        ),
        invariant(
            CrossAxisInvariantKindIR::LanguageSemanticAuthorityAbsent,
            language_non_authoritative,
            vec!["SEMANTIC_AUTHORITY:false".to_string()],
        ),
        invariant(
            CrossAxisInvariantKindIR::PackageDependencyPointsTowardCore,
            package_boundary.validate(),
            vec![package_boundary.boundary_sha256.clone()],
        ),
    ];

    let mut violations = Vec::new();
    for axis in &axes {
        if axis.status != LanguageAxisStatusIR::Pass {
            violations.push(format!("AXIS_{:?}_UNRESOLVED", axis.axis));
        }
    }
    for invariant in &cross_axis_invariants {
        if !invariant.satisfied {
            violations.push(format!("INVARIANT_{:?}_FAILED", invariant.kind));
        }
    }
    for link in &cross_axis_links {
        if !link.satisfied {
            violations.push(format!("LINK_{:?}_FAILED", link.kind));
        }
    }
    let complete = violations.is_empty();
    let mut integration = SixAxisIntegrationIR {
        schema: SIX_AXIS_INTEGRATION_SCHEMA.to_string(),
        request_id: sources.request_id.to_string(),
        turn_index: sources.turn_index,
        axes,
        cross_axis_links,
        cross_axis_invariants,
        violations,
        package_boundary,
        complete,
        semantic_authority: false,
        language_can_execute: false,
        natural_realization_sha256: sources.natural_realization.realization_sha256.clone(),
        integration_sha256: String::new(),
    };
    integration.integration_sha256 = six_axis_integration_sha256(&integration);
    integration
}

pub fn language_cortex_package_boundary_sha256(
    boundary: &LanguageCortexPackageBoundaryIR,
) -> String {
    let mut canonical = boundary.clone();
    canonical.boundary_sha256.clear();
    content_sha256(&canonical)
}

pub fn six_axis_integration_sha256(integration: &SixAxisIntegrationIR) -> String {
    let mut canonical = integration.clone();
    canonical.integration_sha256.clear();
    content_sha256(&canonical)
}

fn required_axes() -> BTreeSet<LanguageAxisIR> {
    BTreeSet::from([
        LanguageAxisIR::GrammaticalComposition,
        LanguageAxisIR::DiscourseTopicState,
        LanguageAxisIR::DeixisEllipsis,
        LanguageAxisIR::PragmaticIntent,
        LanguageAxisIR::PlanResultBoundary,
        LanguageAxisIR::EvidenceGroundedRealization,
    ])
}

fn required_links() -> BTreeSet<CrossAxisLinkKindIR> {
    BTreeSet::from([
        CrossAxisLinkKindIR::GrammaticalCompositionToPragmaticIntent,
        CrossAxisLinkKindIR::DiscourseToReferenceResolution,
        CrossAxisLinkKindIR::ReferenceToPragmaticIntent,
        CrossAxisLinkKindIR::PragmaticIntentToPlan,
        CrossAxisLinkKindIR::PlanResultToNaturalRealization,
        CrossAxisLinkKindIR::DiscourseToNaturalRealization,
        CrossAxisLinkKindIR::ReferenceToNaturalRealization,
        CrossAxisLinkKindIR::NaturalToGroundedRealization,
    ])
}

fn cross_axis_link(
    kind: CrossAxisLinkKindIR,
    source_axis: LanguageAxisIR,
    target_axis: LanguageAxisIR,
    active: bool,
    satisfied: bool,
    evidence_refs: Vec<String>,
) -> CrossAxisLinkIR {
    CrossAxisLinkIR {
        kind,
        source_axis,
        target_axis,
        active,
        satisfied,
        evidence_refs,
    }
}

fn natural_source_ref(realization: &NaturalRealizationIR, kind: &str, value: &str) -> bool {
    let expected = format!("{kind}:{value}");
    realization
        .sentences
        .iter()
        .flat_map(|sentence| sentence.source_refs.iter())
        .any(|reference| reference == &expected)
}

fn natural_source_kind(realization: &NaturalRealizationIR, kind: &str) -> bool {
    let prefix = format!("{kind}:");
    realization
        .sentences
        .iter()
        .flat_map(|sentence| sentence.source_refs.iter())
        .any(|reference| reference.starts_with(&prefix))
}

/// A later continuation question can select an already stored decision gate
/// without materializing a new current-turn intent node.  The realization
/// circuit preserves that contextual selection as typed grounding on the
/// generated meaning graph.  Require both the source turn and gate status so
/// an arbitrary continuation-shaped surface cannot satisfy the cross-axis
/// link by itself.
fn contextual_continuation_resolution_evidence(realization: &NaturalRealizationIR) -> Vec<String> {
    if realization.response_act != NaturalResponseActIR::ContinuationGate {
        return Vec::new();
    }

    let mut evidence = BTreeSet::new();
    for trace in &realization.generation_traces {
        let grounding_refs = trace
            .meaning
            .nodes
            .iter()
            .flat_map(|node| node.grounding_refs.iter())
            .collect::<BTreeSet<_>>();
        let has_source_turn = grounding_refs
            .iter()
            .any(|reference| reference.starts_with("PENDING_GATE:SOURCE_TURN:"));
        let has_status = grounding_refs
            .iter()
            .any(|reference| reference.starts_with("PENDING_GATE:STATUS:"));
        if has_source_turn && has_status {
            evidence.insert(format!(
                "CONTEXTUAL_CONTINUATION_RESOLUTION:{}",
                trace.generation_sha256
            ));
        }
    }
    evidence.into_iter().collect()
}

fn axis<T: Serialize>(
    kind: LanguageAxisIR,
    passed: bool,
    schema: &str,
    component: &T,
    evidence_refs: Vec<String>,
) -> LanguageAxisEvidenceIR {
    LanguageAxisEvidenceIR {
        axis: kind,
        status: if passed {
            LanguageAxisStatusIR::Pass
        } else {
            LanguageAxisStatusIR::Unresolved
        },
        component_schema: schema.to_string(),
        component_sha256: content_sha256(component),
        evidence_refs,
        semantic_authority: false,
        external_action_executed: false,
    }
}

fn invariant(
    kind: CrossAxisInvariantKindIR,
    satisfied: bool,
    evidence_refs: Vec<String>,
) -> CrossAxisInvariantIR {
    CrossAxisInvariantIR {
        kind,
        satisfied,
        evidence_refs,
    }
}

fn composition_evidence(composition: &CompositionalAnalysisIR) -> Vec<String> {
    let mut evidence = composition
        .frames
        .iter()
        .take(MAX_AXIS_EVIDENCE_REFS)
        .map(|frame| frame.frame_id.clone())
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        evidence.push("NO_ACTION_FRAME_REQUIRED".to_string());
    }
    evidence
}

fn discourse_evidence(state: &ConversationStateIR) -> Vec<String> {
    let mut evidence = vec![state.state_sha256.clone()];
    evidence.extend(
        state
            .active_topics
            .iter()
            .take(MAX_AXIS_EVIDENCE_REFS.saturating_sub(1))
            .map(|topic| topic.topic_id.clone()),
    );
    evidence
}

fn reference_evidence(reference: &ReferenceResolutionIR) -> Vec<String> {
    let mut evidence = reference
        .discourse_bindings
        .iter()
        .take(MAX_AXIS_EVIDENCE_REFS)
        .map(|binding| format!("BINDING:{:?}:{}", binding.kind, binding.source_surface))
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        evidence.push(if reference.ambiguous_reference_surfaces.is_empty() {
            "NO_REFERENCE_REQUIRED".to_string()
        } else {
            "REFERENCE_AMBIGUITY_PRESERVED".to_string()
        });
    }
    evidence
}

fn pragmatic_evidence(interpretation: &PragmaticInterpretationIR) -> Vec<String> {
    let mut evidence = vec![
        format!("SPEECH_ACT:{:?}", interpretation.speech_act),
        interpretation.language_center.semantic_sha256.clone(),
        interpretation.language_center.graph_sha256.clone(),
    ];
    if let Some(primary) = &interpretation.pragmatic_intent_graph.primary {
        evidence.push(format!("INTENT:{:?}", primary.kind));
    }
    evidence
}

fn plan_result_evidence(
    boundary: &PlanResultBoundaryIR,
    plan: Option<&PlanIR>,
    provenance: &InteractionProvenanceGraphIR,
) -> Vec<String> {
    let mut evidence = vec![
        boundary.boundary_sha256.clone(),
        provenance.graph_sha256.clone(),
    ];
    if let Some(plan) = plan {
        evidence.push(plan.plan_sha256.clone());
    } else {
        evidence.push("NO_CURRENT_PLAN".to_string());
    }
    evidence
}

fn references_are_consistent(reference: &ReferenceResolutionIR) -> bool {
    let used = reference.used_referent_ids.iter().collect::<BTreeSet<_>>();
    let bound = reference
        .discourse_bindings
        .iter()
        .flat_map(|binding| binding.referent_ids.iter())
        .collect::<BTreeSet<_>>();
    let ambiguous = reference
        .ambiguous_reference_surfaces
        .iter()
        .collect::<BTreeSet<_>>();
    let expected_resolution_count = reference
        .discourse_bindings
        .iter()
        .filter(|binding| binding.kind != DiscourseBindingKindIR::PluralEventMemberReference)
        .count();
    used.len() == reference.used_referent_ids.len()
        && ambiguous.len() == reference.ambiguous_reference_surfaces.len()
        && used == bound
        && reference.resolved_reference_count == expected_resolution_count
}

fn reports_do_not_establish_results(
    ledger: &ActionStateLedgerIR,
    provenance: &InteractionProvenanceGraphIR,
) -> bool {
    provenance.nodes.iter().all(|node| {
        if node.kind != InteractionProvenanceNodeKindIR::VerifiedResult {
            return true;
        }
        let Some(action_id) = node.action_id.as_deref() else {
            return false;
        };
        ledger.evidence_audit_history.iter().any(|audit| {
            audit.action_id == action_id
                && audit.verified_outcome
                && matches!(
                    audit.resulting_execution_status,
                    ActionExecutionStatusIR::Succeeded | ActionExecutionStatusIR::Failed
                )
        })
    })
}

fn has_goal_plan_edge(provenance: &InteractionProvenanceGraphIR) -> bool {
    provenance.edges.iter().any(|edge| {
        edge.relation
            == crate::interaction_provenance::InteractionProvenanceRelationIR::GoalProjectsPlan
    })
}

fn content_sha256<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable integration component");
    format!("{:x}", Sha256::digest(bytes))
}

fn text_sha256(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn manifests_have_one_way_dependency() -> bool {
    LANGUAGE_ADAPTER_MANIFEST.lines().any(|line| {
        let line = line.trim();
        line.starts_with("dockable-semantic-core")
            && line.contains("path = \"../dockable-semantic-core\"")
    }) && !SEMANTIC_CORE_MANIFEST.contains("semantic-core-adapters")
        && !SEMANTIC_CORE_MANIFEST.contains("semantic_core_adapters")
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 4_096
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::{
        ConversationInputModalityIR, ConversationTurnRequestIR, CONVERSATION_TURN_REQUEST_SCHEMA,
    };
    use crate::language_knowledge::LanguageCodeIR;
    use crate::CognitiveApi;

    fn request(id: &str, text: &str) -> ConversationTurnRequestIR {
        ConversationTurnRequestIR {
            schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
            conversation_id: id.to_string(),
            turn_index: 1,
            request_id: format!("{id}-1"),
            modality: ConversationInputModalityIR::Text,
            raw_text: text.to_string(),
            input_confidence_millis: 1_000,
            alternatives: Vec::new(),
            output_language: Some(LanguageCodeIR::English),
            context_tags: Vec::new(),
            max_plan_steps: 16,
        }
    }

    #[test]
    fn package_boundary_is_hash_sealed_and_non_authoritative() {
        let boundary = language_cortex_package_boundary();
        assert!(boundary.validate());
        assert!(!boundary.raw_language_reaches_core);
        assert!(!boundary.adapter_owns_semantic_state);
        assert!(!boundary.semantic_authority);
        assert!(!boundary.external_action_execution_authority);
    }

    #[test]
    fn package_boundary_tampering_is_detected() {
        let mut boundary = language_cortex_package_boundary();
        boundary.raw_language_reaches_core = true;
        assert!(!boundary.validate());
    }

    #[test]
    fn package_boundary_is_bound_to_actual_one_way_cargo_manifests() {
        let boundary = language_cortex_package_boundary();
        assert!(manifests_have_one_way_dependency());
        assert_eq!(
            boundary.language_adapter_manifest_sha256,
            text_sha256(LANGUAGE_ADAPTER_MANIFEST)
        );
        assert_eq!(
            boundary.semantic_core_manifest_sha256,
            text_sha256(SEMANTIC_CORE_MANIFEST)
        );
    }

    #[test]
    fn response_integration_recomputes_from_all_six_live_components() {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let response = api
            .process_conversation_turn(&request(
                "R43_UNIT_LIVE",
                "Do not delete the cache; inspect the log instead.",
            ))
            .expect("conversation response");
        assert!(response.six_axis_integration.complete);
        assert!(response
            .six_axis_integration
            .validate_against(SixAxisIntegrationSources {
                request_id: "R43_UNIT_LIVE-1",
                turn_index: 1,
                pragmatic_interpretation: &response.pragmatic_interpretation,
                conversation_state: &response.conversation_state,
                reference_resolution: &response.reference_resolution,
                action_state_analysis: &response.action_state_analysis,
                plan_result_boundary: &response.plan_result_boundary,
                grounded_plan: response
                    .grounded_response
                    .as_deref()
                    .map(|grounded| &grounded.plan),
                natural_realization: &response.natural_realization,
                grounded_realization: &response.grounded_realization,
                interaction_provenance: &response.interaction_provenance,
                realized_output: &response.output.text,
            }));
    }

    #[test]
    fn rehashed_component_substitution_fails_live_cross_check() {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let response = api
            .process_conversation_turn(&request("R43_UNIT_SUBSTITUTE", "Inspect the queue."))
            .expect("conversation response");
        let mut tampered = response.six_axis_integration.clone();
        tampered.axes[0].component_sha256 = "0".repeat(64);
        tampered.integration_sha256 = six_axis_integration_sha256(&tampered);
        assert!(tampered.validate());
        assert!(!tampered.validate_against(SixAxisIntegrationSources {
            request_id: "R43_UNIT_SUBSTITUTE-1",
            turn_index: 1,
            pragmatic_interpretation: &response.pragmatic_interpretation,
            conversation_state: &response.conversation_state,
            reference_resolution: &response.reference_resolution,
            action_state_analysis: &response.action_state_analysis,
            plan_result_boundary: &response.plan_result_boundary,
            grounded_plan: response
                .grounded_response
                .as_deref()
                .map(|grounded| &grounded.plan),
            natural_realization: &response.natural_realization,
            grounded_realization: &response.grounded_realization,
            interaction_provenance: &response.interaction_provenance,
            realized_output: &response.output.text,
        }));
    }

    #[test]
    fn ambiguous_action_alternative_is_complete_only_because_it_fails_closed() {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let response = api
            .process_conversation_turn(&request(
                "R43_UNIT_AMBIGUOUS",
                "Either inspect the queue or repair the worker.",
            ))
            .expect("conversation response");
        assert!(response.grounded_response.is_none());
        assert!(response.six_axis_integration.complete);
        assert!(response
            .six_axis_integration
            .cross_axis_invariants
            .iter()
            .any(|invariant| {
                invariant.kind == CrossAxisInvariantKindIR::AmbiguityFailsClosed
                    && invariant.satisfied
            }));
    }
}
