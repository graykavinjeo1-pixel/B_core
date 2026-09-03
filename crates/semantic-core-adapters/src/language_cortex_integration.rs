//! Tamper-evident binding for one complete Language Cortex response.
//!
//! This receipt binds every live response component to the request that
//! produced it. It is an integrity and regression boundary only: language and
//! hashes remain non-authoritative, and neither can execute an external action.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::action_state::ActionStateAnalysisIR;
use crate::cognitive::{ConversationalOutputIR, NaturalLanguageResponseIR};
use crate::conditional_guard::ConditionalGuardEvaluationIR;
use crate::conversation::{
    validate_conversation_state, ConversationStateIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DiscourseGroupUpdateIR, NormalizedUtteranceIR,
    ReferenceResolutionIR, TopicTransitionIR, CONVERSATION_FRONTEND_SCHEMA,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use crate::definition_grounding::DefinitionGroundingIR;
use crate::discourse_qa::DiscourseAnswerIR;
use crate::discourse_relations::DialogueRelationAnswerIR;
use crate::grounded_realization::EvidenceGroundedRealizationIR;
use crate::interaction_provenance::InteractionProvenanceGraphIR;
use crate::natural_realization::NaturalRealizationIR;
use crate::plan_result_boundary::PlanResultBoundaryIR;
use crate::pragmatic_memory::{validate_pragmatic_memory_state, PragmaticMemoryStateIR};
use crate::pragmatics::PragmaticInterpretationIR;
use crate::six_axis_integration::{SixAxisIntegrationIR, SixAxisIntegrationSources};
use crate::temporal::TemporalAnswerIR;

pub const LANGUAGE_CORTEX_RESPONSE_INTEGRATION_SCHEMA: &str =
    "B_CORE_LANGUAGE_CORTEX_RESPONSE_INTEGRATION_IR_5";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageCortexResponseIntegrationIR {
    pub schema: String,
    pub conversation_id: String,
    pub request_id: String,
    pub turn_index: u64,
    pub request_sha256: String,
    pub normalization_sha256: String,
    pub definition_grounding_sha256: String,
    pub reference_resolution_sha256: String,
    pub pragmatic_interpretation_sha256: String,
    pub action_state_analysis_sha256: String,
    pub plan_result_boundary_sha256: String,
    pub discourse_outputs_sha256: String,
    pub pragmatic_state_sha256: String,
    pub conversation_state_sha256: String,
    pub natural_realization_sha256: String,
    pub grounded_realization_sha256: String,
    pub interaction_provenance_sha256: String,
    pub six_axis_integration_sha256: String,
    pub output_sha256: String,
    pub response_payload_sha256: String,
    pub unsupported_explanation_facts: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub network_calls: usize,
    pub recursive_source_mutations: usize,
    pub semantic_authority: bool,
    pub language_can_execute: bool,
    pub external_action_executed: bool,
    pub violations: Vec<String>,
    pub complete: bool,
    pub integration_sha256: String,
}

impl LanguageCortexResponseIntegrationIR {
    pub fn validate(&self) -> bool {
        self.schema == LANGUAGE_CORTEX_RESPONSE_INTEGRATION_SCHEMA
            && valid_id(&self.conversation_id)
            && valid_id(&self.request_id)
            && self.turn_index > 0
            && [
                self.request_sha256.as_str(),
                self.normalization_sha256.as_str(),
                self.definition_grounding_sha256.as_str(),
                self.reference_resolution_sha256.as_str(),
                self.pragmatic_interpretation_sha256.as_str(),
                self.action_state_analysis_sha256.as_str(),
                self.plan_result_boundary_sha256.as_str(),
                self.discourse_outputs_sha256.as_str(),
                self.pragmatic_state_sha256.as_str(),
                self.conversation_state_sha256.as_str(),
                self.natural_realization_sha256.as_str(),
                self.grounded_realization_sha256.as_str(),
                self.interaction_provenance_sha256.as_str(),
                self.six_axis_integration_sha256.as_str(),
                self.output_sha256.as_str(),
                self.response_payload_sha256.as_str(),
                self.integration_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_digest)
            && self.unsupported_explanation_facts == 0
            && self.external_llm_calls == 0
            && self.local_teacher_calls == 0
            && self.network_calls == 0
            && self.recursive_source_mutations == 0
            && !self.semantic_authority
            && !self.language_can_execute
            && !self.external_action_executed
            && self.violations.len() <= 32
            && self
                .violations
                .iter()
                .all(|violation| !violation.trim().is_empty() && violation.len() <= 256)
            && self.complete == self.violations.is_empty()
            && self.integration_sha256 == language_cortex_response_integration_sha256(self)
    }

    pub fn validate_against(&self, sources: LanguageCortexResponseSources<'_>) -> bool {
        self.validate() && self == &build_language_cortex_response_integration(sources)
    }
}

#[derive(Clone, Copy)]
pub struct LanguageCortexResponseSources<'a> {
    pub request: &'a ConversationTurnRequestIR,
    pub disposition: ConversationTurnDispositionIR,
    pub normalization: &'a NormalizedUtteranceIR,
    pub definition_grounding: &'a DefinitionGroundingIR,
    pub reference_resolution: &'a ReferenceResolutionIR,
    pub pragmatic_interpretation: &'a PragmaticInterpretationIR,
    pub action_state_analysis: &'a ActionStateAnalysisIR,
    pub plan_result_boundary: &'a PlanResultBoundaryIR,
    pub discourse_group_update: Option<&'a DiscourseGroupUpdateIR>,
    pub topic_transition: Option<&'a TopicTransitionIR>,
    pub pragmatic_state: &'a PragmaticMemoryStateIR,
    pub conversation_state: &'a ConversationStateIR,
    pub grounded_response: Option<&'a NaturalLanguageResponseIR>,
    pub discourse_answer: Option<&'a DiscourseAnswerIR>,
    pub dialogue_relation_answer: Option<&'a DialogueRelationAnswerIR>,
    pub temporal_answer: Option<&'a TemporalAnswerIR>,
    pub conditional_guard_evaluations: &'a [ConditionalGuardEvaluationIR],
    pub natural_realization: &'a NaturalRealizationIR,
    pub grounded_realization: &'a EvidenceGroundedRealizationIR,
    pub interaction_provenance: &'a InteractionProvenanceGraphIR,
    pub six_axis_integration: &'a SixAxisIntegrationIR,
    pub output: &'a ConversationalOutputIR,
}

#[derive(Serialize)]
struct DiscourseOutputsView<'a> {
    discourse_group_update: Option<&'a DiscourseGroupUpdateIR>,
    topic_transition: Option<&'a TopicTransitionIR>,
    grounded_response: Option<&'a NaturalLanguageResponseIR>,
    discourse_answer: Option<&'a DiscourseAnswerIR>,
    dialogue_relation_answer: Option<&'a DialogueRelationAnswerIR>,
    temporal_answer: Option<&'a TemporalAnswerIR>,
    conditional_guard_evaluations: &'a [ConditionalGuardEvaluationIR],
}

#[derive(Serialize)]
struct ResponsePayloadView<'a> {
    conversation_id: &'a str,
    turn_index: u64,
    disposition: ConversationTurnDispositionIR,
    normalization: &'a NormalizedUtteranceIR,
    definition_grounding: &'a DefinitionGroundingIR,
    reference_resolution: &'a ReferenceResolutionIR,
    pragmatic_interpretation: &'a PragmaticInterpretationIR,
    action_state_analysis: &'a ActionStateAnalysisIR,
    plan_result_boundary: &'a PlanResultBoundaryIR,
    discourse_group_update: Option<&'a DiscourseGroupUpdateIR>,
    topic_transition: Option<&'a TopicTransitionIR>,
    pragmatic_state: &'a PragmaticMemoryStateIR,
    conversation_state: &'a ConversationStateIR,
    grounded_response: Option<&'a NaturalLanguageResponseIR>,
    discourse_answer: Option<&'a DiscourseAnswerIR>,
    dialogue_relation_answer: Option<&'a DialogueRelationAnswerIR>,
    temporal_answer: Option<&'a TemporalAnswerIR>,
    conditional_guard_evaluations: &'a [ConditionalGuardEvaluationIR],
    natural_realization: &'a NaturalRealizationIR,
    grounded_realization: &'a EvidenceGroundedRealizationIR,
    interaction_provenance: &'a InteractionProvenanceGraphIR,
    six_axis_integration: &'a SixAxisIntegrationIR,
    output: &'a ConversationalOutputIR,
}

pub fn build_language_cortex_response_integration(
    sources: LanguageCortexResponseSources<'_>,
) -> LanguageCortexResponseIntegrationIR {
    let discourse_outputs = DiscourseOutputsView {
        discourse_group_update: sources.discourse_group_update,
        topic_transition: sources.topic_transition,
        grounded_response: sources.grounded_response,
        discourse_answer: sources.discourse_answer,
        dialogue_relation_answer: sources.dialogue_relation_answer,
        temporal_answer: sources.temporal_answer,
        conditional_guard_evaluations: sources.conditional_guard_evaluations,
    };
    let response_payload = ResponsePayloadView {
        conversation_id: &sources.request.conversation_id,
        turn_index: sources.request.turn_index,
        disposition: sources.disposition,
        normalization: sources.normalization,
        definition_grounding: sources.definition_grounding,
        reference_resolution: sources.reference_resolution,
        pragmatic_interpretation: sources.pragmatic_interpretation,
        action_state_analysis: sources.action_state_analysis,
        plan_result_boundary: sources.plan_result_boundary,
        discourse_group_update: sources.discourse_group_update,
        topic_transition: sources.topic_transition,
        pragmatic_state: sources.pragmatic_state,
        conversation_state: sources.conversation_state,
        grounded_response: sources.grounded_response,
        discourse_answer: sources.discourse_answer,
        dialogue_relation_answer: sources.dialogue_relation_answer,
        temporal_answer: sources.temporal_answer,
        conditional_guard_evaluations: sources.conditional_guard_evaluations,
        natural_realization: sources.natural_realization,
        grounded_realization: sources.grounded_realization,
        interaction_provenance: sources.interaction_provenance,
        six_axis_integration: sources.six_axis_integration,
        output: sources.output,
    };
    let violations = source_component_violations(sources);
    let complete = violations.is_empty();
    let mut integration = LanguageCortexResponseIntegrationIR {
        schema: LANGUAGE_CORTEX_RESPONSE_INTEGRATION_SCHEMA.to_string(),
        conversation_id: sources.request.conversation_id.clone(),
        request_id: sources.request.request_id.clone(),
        turn_index: sources.request.turn_index,
        request_sha256: content_sha256(sources.request),
        normalization_sha256: content_sha256(sources.normalization),
        definition_grounding_sha256: sources.definition_grounding.grounding_sha256.clone(),
        reference_resolution_sha256: content_sha256(sources.reference_resolution),
        pragmatic_interpretation_sha256: content_sha256(sources.pragmatic_interpretation),
        action_state_analysis_sha256: content_sha256(sources.action_state_analysis),
        plan_result_boundary_sha256: sources.plan_result_boundary.boundary_sha256.clone(),
        discourse_outputs_sha256: content_sha256(&discourse_outputs),
        pragmatic_state_sha256: sources.pragmatic_state.state_sha256.clone(),
        conversation_state_sha256: sources.conversation_state.state_sha256.clone(),
        natural_realization_sha256: sources.natural_realization.realization_sha256.clone(),
        grounded_realization_sha256: sources.grounded_realization.realization_sha256.clone(),
        interaction_provenance_sha256: sources.interaction_provenance.graph_sha256.clone(),
        six_axis_integration_sha256: sources.six_axis_integration.integration_sha256.clone(),
        output_sha256: content_sha256(sources.output),
        response_payload_sha256: content_sha256(&response_payload),
        unsupported_explanation_facts: sources.grounded_realization.unsupported_claims,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
        semantic_authority: false,
        language_can_execute: false,
        external_action_executed: false,
        violations,
        complete,
        integration_sha256: String::new(),
    };
    integration.integration_sha256 = language_cortex_response_integration_sha256(&integration);
    integration
}

pub fn language_cortex_response_integration_sha256(
    integration: &LanguageCortexResponseIntegrationIR,
) -> String {
    let mut canonical = integration.clone();
    canonical.integration_sha256.clear();
    content_sha256(&canonical)
}

fn source_component_violations(sources: LanguageCortexResponseSources<'_>) -> Vec<String> {
    let mut violations = Vec::new();
    let request_valid = sources.request.schema == CONVERSATION_TURN_REQUEST_SCHEMA
        && valid_id(&sources.request.conversation_id)
        && valid_id(&sources.request.request_id)
        && sources.request.turn_index > 0
        && !sources.request.raw_text.trim().is_empty()
        && sources.request.max_plan_steps > 0;
    let turn_aligned = sources.normalization.schema == CONVERSATION_FRONTEND_SCHEMA
        && sources.normalization.raw_text == sources.request.raw_text
        && sources.pragmatic_state.conversation_id == sources.request.conversation_id
        && sources.pragmatic_state.completed_turns == sources.request.turn_index
        && sources.conversation_state.conversation_id == sources.request.conversation_id
        && sources.conversation_state.completed_turns == sources.request.turn_index
        && sources.interaction_provenance.conversation_id == sources.request.conversation_id
        && sources.interaction_provenance.current_request_id == sources.request.request_id
        && sources.interaction_provenance.completed_turns == sources.request.turn_index
        && sources.six_axis_integration.request_id == sources.request.request_id
        && sources.six_axis_integration.turn_index == sources.request.turn_index;
    let optional_components_valid = sources
        .discourse_group_update
        .is_none_or(DiscourseGroupUpdateIR::validate)
        && sources
            .topic_transition
            .is_none_or(TopicTransitionIR::validate);
    let six_axis_valid = sources
        .six_axis_integration
        .validate_against(SixAxisIntegrationSources {
            request_id: &sources.request.request_id,
            turn_index: sources.request.turn_index,
            pragmatic_interpretation: sources.pragmatic_interpretation,
            conversation_state: sources.conversation_state,
            reference_resolution: sources.reference_resolution,
            action_state_analysis: sources.action_state_analysis,
            plan_result_boundary: sources.plan_result_boundary,
            grounded_plan: sources.grounded_response.map(|response| &response.plan),
            natural_realization: sources.natural_realization,
            grounded_realization: sources.grounded_realization,
            interaction_provenance: sources.interaction_provenance,
            realized_output: &sources.output.text,
        });
    let checks = [
        (request_valid, "REQUEST_INVALID"),
        (turn_aligned, "TURN_STATE_MISALIGNED"),
        (
            sources.definition_grounding.validate(),
            "DEFINITION_GROUNDING_INVALID",
        ),
        (
            validate_pragmatic_memory_state(sources.pragmatic_state).is_ok(),
            "PRAGMATIC_STATE_INVALID",
        ),
        (
            validate_conversation_state(sources.conversation_state).is_ok(),
            "CONVERSATION_STATE_INVALID",
        ),
        (
            optional_components_valid,
            "OPTIONAL_DISCOURSE_OUTPUT_INVALID",
        ),
        (
            !sources.action_state_analysis.semantic_authority
                && !sources.action_state_analysis.external_action_executed,
            "ACTION_STATE_AUTHORITY_VIOLATION",
        ),
        (
            sources.plan_result_boundary.validate_against(
                &sources.normalization.semantic_surface_text,
                sources.action_state_analysis,
                &sources.conversation_state.action_state_ledger,
            ),
            "PLAN_RESULT_BOUNDARY_INVALID",
        ),
        (
            sources.natural_realization.validate_output(
                sources.output.language,
                &sources.output.text,
                sources.output.unsupported_freeform_claims,
            ),
            "NATURAL_REALIZATION_INVALID",
        ),
        (
            sources.grounded_realization.validate(),
            "GROUNDED_REALIZATION_INVALID",
        ),
        (
            sources.interaction_provenance.validate_against(
                sources.grounded_realization,
                &sources.conversation_state.action_state_ledger,
            ),
            "INTERACTION_PROVENANCE_INVALID",
        ),
        (six_axis_valid, "SIX_AXIS_LIVE_BINDING_INVALID"),
        (sources.six_axis_integration.complete, "SIX_AXIS_INCOMPLETE"),
        (
            sources.output.language == sources.natural_realization.language
                && sources.output.text == sources.natural_realization.realized_text
                && sources.output.language == sources.grounded_realization.language
                && sources.output.text == sources.grounded_realization.realized_text
                && sources.output.unsupported_freeform_claims
                    == sources.grounded_realization.unsupported_claims,
            "OUTPUT_REALIZATION_MISMATCH",
        ),
        (
            sources.output.unsupported_freeform_claims == 0,
            "UNSUPPORTED_EXPLANATION_FACTS_PRESENT",
        ),
    ];
    violations.extend(
        checks
            .into_iter()
            .filter(|(passed, _)| !passed)
            .map(|(_, violation)| violation.to_string()),
    );
    violations
}

fn content_sha256<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("bounded response component serializes");
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 320
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::{ConversationInputModalityIR, ConversationTurnRequestIR};
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
    fn complete_response_is_bound_to_its_live_request_and_components() {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let request = request(
            "R50_UNIT_LIVE",
            "Inspect the cache and if it is stale, repair it.",
        );
        let response = api
            .process_conversation_turn(&request)
            .expect("conversation response");
        assert!(response.language_cortex_integration.complete);
        assert!(response.validate_against(&request));
    }

    #[test]
    fn output_or_request_substitution_breaks_full_response_validation() {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let source_request = request("R50_UNIT_TAMPER", "Inspect the queue.");
        let response = api
            .process_conversation_turn(&source_request)
            .expect("conversation response");
        let mut tampered = response.clone();
        tampered.output.text.push_str(" unsupported");
        assert!(!tampered.validate_against(&source_request));
        let substitute = request("R50_UNIT_SUBSTITUTE", "Inspect the queue.");
        assert!(!response.validate_against(&substitute));
    }

    #[test]
    fn receipt_rehash_cannot_hide_a_live_component_substitution() {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let request = request("R50_UNIT_REHASH", "Inspect the cache.");
        let response = api
            .process_conversation_turn(&request)
            .expect("conversation response");
        let mut tampered = response.clone();
        tampered.conversation_state.completed_turns += 1;
        tampered.language_cortex_integration.integration_sha256 =
            language_cortex_response_integration_sha256(&tampered.language_cortex_integration);
        assert!(tampered.language_cortex_integration.validate());
        assert!(!tampered.validate_against(&request));
    }
}
