//! Evidence-bound document planning for standalone B_Core responses.
//!
//! The plan is language independent apart from the requested output language.
//! It selects discourse and presentation operators from an already approved
//! response.  It never imports facts from the source utterance or invents a
//! prose claim.  Rendering and structural interpretation share the same block
//! vocabulary so formatted output remains inspectable without an LLM.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::approved_response::{
    approved_event_predicate_spec, ApprovedCompositionalClaimIR, ApprovedCompositionalResponseIR,
    ApprovedConfigurationStateIR,
    ApprovedDiscourseRelationIR, ApprovedEventFocusIR, ApprovedEventInformationRoleIR,
    ApprovedEventInformationStructureIR, ApprovedEventPerspectiveIR, ApprovedEventPhaseIR,
    ApprovedEventPredicateSenseIR, ApprovedEventRealizationClassIR, ApprovedEventVoiceIR,
    ApprovedLexicalNodeIR, ApprovedModalityIR, ApprovedOpenValueIR, ApprovedOperationIR,
    ApprovedRelationTypeIR, ApprovedSemanticTypeIR, ApprovedSpeechActIR, ApprovedStatusFrameIR,
    ApprovedVerbosityIR,
};
use crate::language_knowledge::{LanguageCodeIR, LanguageRegisterIR};

pub const DOCUMENT_RESPONSE_PLAN_SCHEMA: &str = "B_CORE_DOCUMENT_RESPONSE_PLAN_IR_9";
pub const DOCUMENT_RESPONSE_OUTPUT_SCHEMA: &str = "B_CORE_DOCUMENT_RESPONSE_OUTPUT_IR_11";
pub const DOCUMENT_SURFACE_STRUCTURE_SCHEMA: &str = "B_CORE_DOCUMENT_SURFACE_STRUCTURE_IR_1";
pub const DOCUMENT_SEMANTIC_INTERPRETATION_SCHEMA: &str =
    "B_CORE_DOCUMENT_SEMANTIC_INTERPRETATION_IR_9";

const MAX_BLOCKS: usize = 32;
const MAX_SECTIONS: usize = 8;
// A compact report still needs semantic grouping, but one heading per claim
// turns a six-fact update into a form rather than readable prose.  Long
// documents use their own 3–4 section planner below; this bound applies only
// to the intermediate range.
const MAX_COMPACT_SECTIONS: usize = 4;
const MAX_CHART_ITEMS: usize = 12;
const MAX_FUSED_CLAUSES: usize = 3;
const MAX_EVENT_ARGUMENTS: usize = 12;
const MAX_RENDERED_CHARS: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentResponseBlockKindIR {
    Lead,
    SectionHeading,
    SubsectionHeading,
    Paragraph,
    OrderedList,
    Table,
    Chart,
    Closing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentResponseRoleIR {
    Orientation,
    CoreContent,
    Correction,
    Comparison,
    Cause,
    Condition,
    Timeline,
    EventFrame,
    EvidenceBoundary,
    Summary,
    Identity,
    Status,
    PlaceAndAccess,
    Measurement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentRhetoricalMoveIR {
    Assertion,
    Correction,
    Cause,
    Condition,
    Support,
    Conclusion,
    Concurrent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRhetoricalUnitIR {
    pub unit_index: usize,
    pub proposition_id: String,
    pub rhetorical_move: DocumentRhetoricalMoveIR,
    #[serde(default)]
    pub predecessor_indices: Vec<usize>,
    pub semantic_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentClauseFusionKindIR {
    SharedSubjectCoordination,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentClauseFusionIR {
    pub fusion_index: usize,
    pub left_proposition_id: String,
    pub right_proposition_id: String,
    pub kind: DocumentClauseFusionKindIR,
    #[serde(default)]
    pub predecessor_fusion_indices: Vec<usize>,
    pub semantic_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentEventPredicateKindIR {
    HostedOccurrence,
    ScheduledProcess,
    Departure,
    Arrival,
    Motion,
    Transfer,
    Creation,
    Inspection,
    StateChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentEventArgumentRoleIR {
    Agent,
    Patient,
    Theme,
    Source,
    Destination,
    Target,
    Date,
    Time,
    Location,
    Instrument,
    Manner,
    InitialState,
    ResultState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentEventArgumentIR {
    pub proposition_id: String,
    pub role: DocumentEventArgumentRoleIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentEventPredicateIR {
    pub predicate_index: usize,
    pub subject_node_id: String,
    pub kind: DocumentEventPredicateKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicate_sense: Option<ApprovedEventPredicateSenseIR>,
    pub phase: ApprovedEventPhaseIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perspective: Option<ApprovedEventPerspectiveIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<ApprovedEventVoiceIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub information_structure: Option<ApprovedEventInformationStructureIR>,
    pub arguments: Vec<DocumentEventArgumentIR>,
    #[serde(default)]
    pub predecessor_predicate_indices: Vec<usize>,
    pub semantic_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApprovedDocumentEventPredicate {
    kind: DocumentEventPredicateKindIR,
    phase: ApprovedEventPhaseIR,
    predicate_sense: Option<ApprovedEventPredicateSenseIR>,
    perspective: Option<ApprovedEventPerspectiveIR>,
    voice: Option<ApprovedEventVoiceIR>,
    information_structure: Option<ApprovedEventInformationStructureIR>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedEventInformationStructure {
    voice: ApprovedEventVoiceIR,
    topic: Option<ApprovedEventInformationRoleIR>,
    focus: ApprovedEventInformationRoleIR,
    omitted_roles: Vec<ApprovedRelationTypeIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentResponseBlockIR {
    pub block_index: usize,
    pub kind: DocumentResponseBlockKindIR,
    pub role: DocumentResponseRoleIR,
    #[serde(default)]
    pub claim_ids: Vec<String>,
    #[serde(default)]
    pub predecessor_indices: Vec<usize>,
    pub semantic_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentResponseSectionIR {
    pub section_index: usize,
    pub role: DocumentResponseRoleIR,
    pub claim_ids: Vec<String>,
    pub block_indices: Vec<usize>,
    #[serde(default)]
    pub predecessor_section_indices: Vec<usize>,
    pub semantic_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentResponsePlanIR {
    pub schema: String,
    pub source_response_sha256: String,
    pub output_language: LanguageCodeIR,
    pub blocks: Vec<DocumentResponseBlockIR>,
    pub sections: Vec<DocumentResponseSectionIR>,
    pub rhetorical_units: Vec<DocumentRhetoricalUnitIR>,
    pub event_predicates: Vec<DocumentEventPredicateIR>,
    pub clause_fusions: Vec<DocumentClauseFusionIR>,
    pub covered_claim_ids: Vec<String>,
    pub unsupported_claims: usize,
    pub semantic_authority: bool,
    pub plan_sha256: String,
}

impl DocumentResponsePlanIR {
    pub fn validate(&self, response: &ApprovedCompositionalResponseIR) -> bool {
        if self.schema != DOCUMENT_RESPONSE_PLAN_SCHEMA
            || !response.validate()
            || self.source_response_sha256 != response.semantic_sha256
            || !matches!(
                self.output_language,
                LanguageCodeIR::Korean | LanguageCodeIR::English
            )
            || self.blocks.is_empty()
            || self.blocks.len() > MAX_BLOCKS
            || self.sections.is_empty()
            || self.sections.len() > MAX_SECTIONS
            || self.unsupported_claims != 0
            || self.semantic_authority
            || self.plan_sha256 != document_response_plan_sha256(self)
        {
            return false;
        }
        // Explicit procedure-order claims are rendered through adjacent step
        // markers, so they are recovered semantically without becoming their
        // own document block or heading.
        let approved_order = response
            .claims
            .iter()
            .filter(|claim| !is_directive_procedure_dependency(claim, response))
            .map(|claim| claim.proposition_id.clone())
            .collect::<Vec<_>>();
        let approved = approved_order
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let section_order = self
            .sections
            .iter()
            .flat_map(|section| section.claim_ids.iter().cloned())
            .collect::<Vec<_>>();
        let primary_order = self
            .blocks
            .iter()
            .filter(|block| is_primary_content(block.kind))
            .flat_map(|block| block.claim_ids.iter().cloned())
            .collect::<Vec<_>>();
        let planned = self
            .blocks
            .iter()
            .flat_map(|block| block.claim_ids.iter().map(String::as_str))
            .collect::<BTreeSet<_>>();
        if self.covered_claim_ids != approved_order
            || section_order != approved_order
            || primary_order != approved_order
            || approved != planned
            || self.rhetorical_units != plan_rhetorical_units(response)
            || self.event_predicates
                != plan_event_predicates(response, &self.sections, &self.rhetorical_units)
            || self.clause_fusions
                != plan_clause_fusions(
                    response,
                    &self.sections,
                    &self.rhetorical_units,
                    &self.event_predicates,
                    self.output_language,
                )
        {
            return false;
        }
        let blocks_valid = self.blocks.iter().enumerate().all(|(index, block)| {
            let block_claims = block
                .claim_ids
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            block.block_index == index
                && !block.semantic_authority
                && block_claims.len() == block.claim_ids.len()
                && block_claims.is_subset(&approved)
                && block.predecessor_indices
                    == (index > 0)
                        .then(|| index - 1)
                        .into_iter()
                        .collect::<Vec<_>>()
                && (matches!(
                    block.kind,
                    DocumentResponseBlockKindIR::Lead
                        | DocumentResponseBlockKindIR::SectionHeading
                        | DocumentResponseBlockKindIR::SubsectionHeading
                        | DocumentResponseBlockKindIR::Closing
                ) == block.claim_ids.is_empty())
                && (block.kind != DocumentResponseBlockKindIR::Chart
                    || block.claim_ids.len() >= 2
                        && block.claim_ids.len() <= MAX_CHART_ITEMS
                        && chart_claims(response, &block.claim_ids).is_some())
        });
        if !blocks_valid {
            return false;
        }

        let framing = [
            DocumentResponseBlockKindIR::Lead,
            DocumentResponseBlockKindIR::SectionHeading,
            DocumentResponseBlockKindIR::Closing,
        ]
        .map(|kind| {
            self.blocks
                .iter()
                .filter(|block| block.kind == kind)
                .count()
        });
        if !framing.iter().all(|count| *count <= 1)
            || !(framing == [0, 0, 0] || framing == [1, 1, 1])
            || (self.sections.len() > 1 && framing != [1, 1, 1])
        {
            return false;
        }

        let mut assigned_blocks = BTreeSet::new();
        let sections_valid = self.sections.iter().enumerate().all(|(index, section)| {
            let section_claims = section
                .claim_ids
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            let ordered_blocks = section
                .block_indices
                .windows(2)
                .all(|window| window[0] < window[1]);
            let section_blocks = section
                .block_indices
                .iter()
                .filter_map(|block_index| self.blocks.get(*block_index))
                .collect::<Vec<_>>();
            let primary_count = section_blocks
                .iter()
                .filter(|block| is_primary_content(block.kind))
                .count();
            let subsection_count = section_blocks
                .iter()
                .filter(|block| block.kind == DocumentResponseBlockKindIR::SubsectionHeading)
                .count();
            let chart_count = section_blocks
                .iter()
                .filter(|block| block.kind == DocumentResponseBlockKindIR::Chart)
                .count();
            section.section_index == index
                && !section.semantic_authority
                && !section.claim_ids.is_empty()
                && section_claims.len() == section.claim_ids.len()
                && section_claims.is_subset(&approved)
                && !section.block_indices.is_empty()
                && ordered_blocks
                && section_blocks.len() == section.block_indices.len()
                && section
                    .block_indices
                    .iter()
                    .all(|block_index| assigned_blocks.insert(*block_index))
                && section.predecessor_section_indices
                    == (index > 0)
                        .then(|| index - 1)
                        .into_iter()
                        .collect::<Vec<_>>()
                && primary_count == 1
                && subsection_count == usize::from(self.sections.len() > 1)
                && chart_count <= 1
                && section_blocks.iter().all(|block| {
                    block.role == section.role
                        && (block.kind == DocumentResponseBlockKindIR::SubsectionHeading
                            && block.claim_ids.is_empty()
                            || block.claim_ids == section.claim_ids)
                })
        });
        sections_valid
            && self.blocks.iter().enumerate().all(|(index, block)| {
                assigned_blocks.contains(&index)
                    || matches!(
                        block.kind,
                        DocumentResponseBlockKindIR::Lead
                            | DocumentResponseBlockKindIR::SectionHeading
                            | DocumentResponseBlockKindIR::Closing
                    )
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentSurfaceNodeKindIR {
    Heading,
    Paragraph,
    OrderedItem,
    TableHeader,
    TableRow,
    Chart,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSurfaceNodeIR {
    pub node_index: usize,
    pub kind: DocumentSurfaceNodeKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<usize>,
    pub surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSurfaceStructureIR {
    pub schema: String,
    pub nodes: Vec<DocumentSurfaceNodeIR>,
    pub structure_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentSemanticEvidenceKindIR {
    Paragraph,
    OrderedItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSemanticClaimIR {
    pub proposition_id: String,
    pub subject: ApprovedLexicalNodeIR,
    pub relation: ApprovedRelationTypeIR,
    pub value: ApprovedOpenValueIR,
    pub polarity: bool,
    pub modality: ApprovedModalityIR,
    pub evidence_kind: DocumentSemanticEvidenceKindIR,
    pub source_node_index: usize,
    pub source_surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSemanticInterpretationIR {
    pub schema: String,
    pub source_response_sha256: String,
    pub output_language: LanguageCodeIR,
    pub claims: Vec<DocumentSemanticClaimIR>,
    pub recovered_claim_ids: Vec<String>,
    pub unsupported_surfaces: Vec<String>,
    pub ambiguous_surface_claims: usize,
    pub semantic_authority: bool,
    pub interpretation_sha256: String,
}

impl DocumentSemanticInterpretationIR {
    pub fn validate(&self, response: &ApprovedCompositionalResponseIR) -> bool {
        if self.schema != DOCUMENT_SEMANTIC_INTERPRETATION_SCHEMA
            || !response.validate()
            || self.source_response_sha256 != response.semantic_sha256
            || !matches!(
                self.output_language,
                LanguageCodeIR::Korean | LanguageCodeIR::English
            )
            || self.claims.len() != response.claims.len()
            || self.recovered_claim_ids.len() != response.claims.len()
            || !self.unsupported_surfaces.is_empty()
            || self.ambiguous_surface_claims != 0
            || self.semantic_authority
            || self.interpretation_sha256 != document_semantic_interpretation_sha256(self)
        {
            return false;
        }
        let approved_ids = response
            .claims
            .iter()
            .map(|claim| claim.proposition_id.as_str())
            .collect::<BTreeSet<_>>();
        let recovered_ids = self
            .recovered_claim_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        approved_ids == recovered_ids
            && recovered_ids.len() == self.recovered_claim_ids.len()
            && self.claims.iter().all(|interpreted| {
                response
                    .claims
                    .iter()
                    .find(|claim| claim.proposition_id == interpreted.proposition_id)
                    .is_some_and(|claim| interpreted_matches_claim(interpreted, claim, response))
            })
    }
}

impl DocumentSurfaceStructureIR {
    pub fn validate(&self) -> bool {
        !self.nodes.is_empty()
            && self.schema == DOCUMENT_SURFACE_STRUCTURE_SCHEMA
            && self.nodes.iter().enumerate().all(|(index, node)| {
                node.node_index == index
                    && !node.surface.trim().is_empty()
                    && ((node.kind == DocumentSurfaceNodeKindIR::OrderedItem)
                        == node.ordinal.is_some())
            })
            && self.structure_sha256 == document_surface_structure_sha256(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentResponseOutputIR {
    pub schema: String,
    pub plan: DocumentResponsePlanIR,
    pub markdown: String,
    pub interpreted_structure: DocumentSurfaceStructureIR,
    pub semantic_interpretation: DocumentSemanticInterpretationIR,
    pub unsupported_claims: usize,
    pub output_sha256: String,
}

impl DocumentResponseOutputIR {
    pub fn validate(&self, response: &ApprovedCompositionalResponseIR) -> bool {
        if self.schema != DOCUMENT_RESPONSE_OUTPUT_SCHEMA
            || !self.plan.validate(response)
            || self.markdown.trim().is_empty()
            || self.markdown.chars().count() > MAX_RENDERED_CHARS
            || self.markdown != render_plan(&self.plan, response)
            || !self.interpreted_structure.validate()
            || interpret_document_surface(&self.markdown).as_ref()
                != Ok(&self.interpreted_structure)
            || !self.semantic_interpretation.validate(response)
            || interpret_document_semantics(&self.markdown, response, self.plan.output_language)
                .as_ref()
                != Ok(&self.semantic_interpretation)
            || self.unsupported_claims != 0
            || self.output_sha256 != document_response_output_sha256(self)
        {
            return false;
        }
        true
    }
}

pub fn build_document_response_plan(
    response: &ApprovedCompositionalResponseIR,
    output_language: LanguageCodeIR,
) -> Result<DocumentResponsePlanIR, String> {
    if !response.validate()
        || !matches!(
            output_language,
            LanguageCodeIR::Korean | LanguageCodeIR::English
        )
    {
        return Err("INVALID_APPROVED_DOCUMENT_RESPONSE".into());
    }
    let claim_ids = response
        .claims
        .iter()
        .filter(|claim| !is_directive_procedure_dependency(claim, response))
        .map(|claim| claim.proposition_id.clone())
        .collect::<Vec<_>>();
    let section_specs = plan_document_sections(response);
    let mut blocks = Vec::new();
    let mut sections = Vec::new();
    let event_claims = response.claims.iter().collect::<Vec<_>>();
    let single_event_response = response.event_realizations.len() == 1
        && response.claims.first().is_some_and(|first| {
            response.claims.iter().all(|claim| {
                claim.subject.node_id == response.event_realizations[0].subject_node_id
                    && is_typed_event_argument(claim, response)
            }) && event_argument_set_complete(first, &event_claims, response)
        });
    let multi_section = section_specs.len() > 1;
    let explanatory = response.style.verbosity == ApprovedVerbosityIR::Explanatory
        || response.speech_act == crate::approved_response::ApprovedSpeechActIR::Explain
        || (response.claims.len() > 2 && !single_event_response);
    if explanatory {
        let discourse_role = role_for(response.discourse_relation);
        let heading_role = if multi_section
            && section_specs
                .iter()
                .any(|section| section.role == discourse_role)
        {
            DocumentResponseRoleIR::CoreContent
        } else {
            discourse_role
        };
        push_block(
            &mut blocks,
            DocumentResponseBlockKindIR::Lead,
            DocumentResponseRoleIR::Orientation,
            Vec::new(),
        );
        push_block(
            &mut blocks,
            DocumentResponseBlockKindIR::SectionHeading,
            heading_role,
            Vec::new(),
        );
    }
    for (section_index, section) in section_specs.into_iter().enumerate() {
        let mut block_indices = Vec::new();
        if multi_section {
            block_indices.push(push_block(
                &mut blocks,
                DocumentResponseBlockKindIR::SubsectionHeading,
                section.role,
                Vec::new(),
            ));
        }
        let chartable = section.claim_ids.len() <= MAX_CHART_ITEMS
            && chart_claims(response, &section.claim_ids).is_some();
        let content_kind = if (response.discourse_relation
            == ApprovedDiscourseRelationIR::Comparison
            && section.claim_ids.len() >= 2)
            || chartable
        {
            DocumentResponseBlockKindIR::Table
        } else if response.claims.len() >= 20 {
            // A true long-form response is continuous prose organized into
            // thematic sections. Rendering every unrelated subject as a
            // numbered list produces a catalogue and restarts numbering in
            // every section. Sentence-level semantic recovery remains
            // unchanged inside the paragraph.
            DocumentResponseBlockKindIR::Paragraph
        } else if claims_share_subject(response, &section.claim_ids)
            || section.claim_ids.len() == 1 && !explanatory
        {
            DocumentResponseBlockKindIR::Paragraph
        } else {
            DocumentResponseBlockKindIR::OrderedList
        };
        block_indices.push(push_block(
            &mut blocks,
            content_kind,
            section.role,
            section.claim_ids.clone(),
        ));
        if chartable {
            block_indices.push(push_block(
                &mut blocks,
                DocumentResponseBlockKindIR::Chart,
                section.role,
                section.claim_ids.clone(),
            ));
        }
        sections.push(DocumentResponseSectionIR {
            section_index,
            role: section.role,
            claim_ids: section.claim_ids,
            block_indices,
            predecessor_section_indices: (section_index > 0)
                .then(|| section_index - 1)
                .into_iter()
                .collect(),
            semantic_authority: false,
        });
    }
    if explanatory {
        push_block(
            &mut blocks,
            DocumentResponseBlockKindIR::Closing,
            DocumentResponseRoleIR::EvidenceBoundary,
            Vec::new(),
        );
    }
    let rhetorical_units = plan_rhetorical_units(response);
    let event_predicates = plan_event_predicates(response, &sections, &rhetorical_units);
    let clause_fusions = plan_clause_fusions(
        response,
        &sections,
        &rhetorical_units,
        &event_predicates,
        output_language,
    );
    let mut plan = DocumentResponsePlanIR {
        schema: DOCUMENT_RESPONSE_PLAN_SCHEMA.into(),
        source_response_sha256: response.semantic_sha256.clone(),
        output_language,
        blocks,
        sections,
        rhetorical_units,
        event_predicates,
        clause_fusions,
        covered_claim_ids: claim_ids,
        unsupported_claims: 0,
        semantic_authority: false,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = document_response_plan_sha256(&plan);
    plan.validate(response)
        .then_some(plan)
        .ok_or_else(|| "DOCUMENT_RESPONSE_PLAN_VALIDATION_FAILED".into())
}

pub fn realize_document_response(
    response: &ApprovedCompositionalResponseIR,
    output_language: LanguageCodeIR,
) -> Result<DocumentResponseOutputIR, String> {
    let plan = build_document_response_plan(response, output_language)?;
    let markdown = render_plan(&plan, response);
    let interpreted_structure = interpret_document_surface(&markdown)?;
    let semantic_interpretation =
        interpret_document_semantics(&markdown, response, output_language)?;
    let mut output = DocumentResponseOutputIR {
        schema: DOCUMENT_RESPONSE_OUTPUT_SCHEMA.into(),
        plan,
        markdown,
        interpreted_structure,
        semantic_interpretation,
        unsupported_claims: 0,
        output_sha256: String::new(),
    };
    output.output_sha256 = document_response_output_sha256(&output);
    output
        .validate(response)
        .then_some(output)
        .ok_or_else(|| "DOCUMENT_RESPONSE_OUTPUT_VALIDATION_FAILED".to_string())
}

pub fn interpret_document_surface(markdown: &str) -> Result<DocumentSurfaceStructureIR, String> {
    if markdown.trim().is_empty() || markdown.chars().count() > MAX_RENDERED_CHARS {
        return Err("INVALID_DOCUMENT_SURFACE".into());
    }
    let mut nodes = Vec::new();
    let mut in_chart = false;
    let mut chart = String::new();
    let mut table_line = 0usize;
    for raw in markdown.lines() {
        let line = raw.trim();
        if line.is_empty() {
            table_line = 0;
            continue;
        }
        if line == "```mermaid" {
            in_chart = true;
            chart.clear();
            continue;
        }
        if in_chart {
            if line == "```" {
                push_surface_node(&mut nodes, DocumentSurfaceNodeKindIR::Chart, None, &chart);
                in_chart = false;
            } else {
                if !chart.is_empty() {
                    chart.push('\n');
                }
                chart.push_str(line);
            }
            continue;
        }
        if line.starts_with('#') {
            push_surface_node(
                &mut nodes,
                DocumentSurfaceNodeKindIR::Heading,
                None,
                line.trim_start_matches('#').trim(),
            );
            continue;
        }
        if line.starts_with('|') && line.ends_with('|') {
            if line
                .chars()
                .all(|character| matches!(character, '|' | '-' | ':' | ' '))
            {
                continue;
            }
            let kind = if table_line == 0 {
                DocumentSurfaceNodeKindIR::TableHeader
            } else {
                DocumentSurfaceNodeKindIR::TableRow
            };
            push_surface_node(&mut nodes, kind, None, line);
            table_line += 1;
            continue;
        }
        if let Some((ordinal, surface)) = parse_ordered_item(line) {
            push_surface_node(
                &mut nodes,
                DocumentSurfaceNodeKindIR::OrderedItem,
                Some(ordinal),
                surface,
            );
            continue;
        }
        push_surface_node(&mut nodes, DocumentSurfaceNodeKindIR::Paragraph, None, line);
    }
    if in_chart || nodes.is_empty() {
        return Err("INVALID_DOCUMENT_SURFACE_STRUCTURE".into());
    }
    let mut structure = DocumentSurfaceStructureIR {
        schema: DOCUMENT_SURFACE_STRUCTURE_SCHEMA.into(),
        nodes,
        structure_sha256: String::new(),
    };
    structure.structure_sha256 = document_surface_structure_sha256(&structure);
    structure
        .validate()
        .then_some(structure)
        .ok_or_else(|| "DOCUMENT_SURFACE_STRUCTURE_VALIDATION_FAILED".into())
}

pub fn interpret_document_semantics(
    markdown: &str,
    response: &ApprovedCompositionalResponseIR,
    output_language: LanguageCodeIR,
) -> Result<DocumentSemanticInterpretationIR, String> {
    if !response.validate()
        || !matches!(
            output_language,
            LanguageCodeIR::Korean | LanguageCodeIR::English
        )
    {
        return Err("INVALID_DOCUMENT_SEMANTIC_CONTEXT".into());
    }
    let structure = interpret_document_surface(markdown)?;
    let mut parsed = Vec::new();
    let mut unsupported_surfaces = Vec::new();
    let mut ambiguous_surface_claims = 0usize;

    for node in &structure.nodes {
        let evidence_kind = match node.kind {
            DocumentSurfaceNodeKindIR::Paragraph => {
                if node.surface
                    == lead_surface_for_response(output_language, response.style.register, response)
                    || node.surface == closing_surface(output_language, response.style.register)
                    || allowed_speech_act_marker(&node.surface, response)
                {
                    continue;
                }
                DocumentSemanticEvidenceKindIR::Paragraph
            }
            DocumentSurfaceNodeKindIR::OrderedItem => DocumentSemanticEvidenceKindIR::OrderedItem,
            DocumentSurfaceNodeKindIR::Heading
            | DocumentSurfaceNodeKindIR::TableHeader
            | DocumentSurfaceNodeKindIR::TableRow
            | DocumentSurfaceNodeKindIR::Chart => continue,
        };
        match parse_semantic_sentence_sequence(&node.surface, response, output_language) {
            SemanticParseResult::Parsed(facts) => {
                parsed.extend(
                    facts
                        .into_iter()
                        .map(|(surface, fact)| (surface, fact, evidence_kind, node.node_index)),
                );
            }
            SemanticParseResult::Unsupported => {
                unsupported_surfaces.push(node.surface.clone());
            }
            SemanticParseResult::Ambiguous => {
                ambiguous_surface_claims += 1;
            }
        }
    }

    let mut used = BTreeSet::new();
    let mut claims = Vec::new();
    for (surface, fact, evidence_kind, source_node_index) in parsed {
        let matches = response
            .claims
            .iter()
            .filter(|claim| {
                !used.contains(claim.proposition_id.as_str())
                    && parsed_matches_claim(&fact, claim, response)
            })
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            let claim = matches[0];
            used.insert(claim.proposition_id.clone());
            claims.push(DocumentSemanticClaimIR {
                proposition_id: claim.proposition_id.clone(),
                subject: fact.subject,
                relation: fact.relation,
                value: fact.value,
                polarity: fact.polarity,
                modality: claim.modality,
                evidence_kind,
                source_node_index,
                source_surface: surface,
            });
        } else if matches.is_empty() {
            unsupported_surfaces.push(surface);
        } else {
            ambiguous_surface_claims += 1;
        }
    }

    // A procedure dependency is recoverable only from two visible, adjacent
    // ordered steps carrying the exact structural sequence markers selected
    // from that dependency.  This prevents list order alone from inventing an
    // `EarlierThan` relation and keeps the relation independent of labels.
    let mut recovered_dependencies = Vec::new();
    for dependency in response
        .claims
        .iter()
        .filter(|claim| claim.relation == ApprovedRelationTypeIR::EarlierThan && claim.polarity)
    {
        if claims
            .iter()
            .any(|claim| claim.proposition_id == dependency.proposition_id)
        {
            continue;
        }
        let ApprovedOpenValueIR::Lexical(successor_event) = &dependency.value else {
            continue;
        };
        let predecessor_action = response.claims.iter().find(|candidate| {
            candidate.relation == ApprovedRelationTypeIR::Action
                && candidate.modality == ApprovedModalityIR::Directive
                && typed_action_event_node_id(candidate, response)
                    .is_some_and(|event_id| event_id == dependency.subject.node_id)
        });
        let successor_action = response.claims.iter().find(|candidate| {
            candidate.relation == ApprovedRelationTypeIR::Action
                && candidate.modality == ApprovedModalityIR::Directive
                && typed_action_event_node_id(candidate, response)
                    .is_some_and(|event_id| event_id == successor_event.node_id)
        });
        let (Some(predecessor_action), Some(successor_action)) =
            (predecessor_action, successor_action)
        else {
            continue;
        };
        let predecessor_evidence = claims
            .iter()
            .find(|claim| claim.proposition_id == predecessor_action.proposition_id);
        let successor_evidence = claims
            .iter()
            .find(|claim| claim.proposition_id == successor_action.proposition_id);
        let (Some(predecessor_evidence), Some(successor_evidence)) =
            (predecessor_evidence, successor_evidence)
        else {
            continue;
        };
        let (Some(predecessor_node), Some(successor_node)) = (
            structure.nodes.get(predecessor_evidence.source_node_index),
            structure.nodes.get(successor_evidence.source_node_index),
        ) else {
            continue;
        };
        let ordered_adjacent = predecessor_node.kind == DocumentSurfaceNodeKindIR::OrderedItem
            && successor_node.kind == DocumentSurfaceNodeKindIR::OrderedItem
            && predecessor_node
                .ordinal
                .zip(successor_node.ordinal)
                .is_some_and(|(predecessor, successor)| successor == predecessor + 1);
        let predecessor_marker = procedure_sequence_marker(
            predecessor_action,
            response,
            output_language,
        );
        let successor_marker = procedure_sequence_marker(successor_action, response, output_language);
        let marked = predecessor_marker
            .is_some_and(|marker| predecessor_evidence.source_surface.starts_with(marker))
            && successor_marker
                .is_some_and(|marker| successor_evidence.source_surface.starts_with(marker));
        if ordered_adjacent && marked {
            let (value, polarity) = effective_claim_value(dependency, response);
            recovered_dependencies.push(DocumentSemanticClaimIR {
                proposition_id: dependency.proposition_id.clone(),
                subject: dependency.subject.clone(),
                relation: dependency.relation,
                value,
                polarity,
                modality: dependency.modality,
                evidence_kind: DocumentSemanticEvidenceKindIR::OrderedItem,
                source_node_index: successor_evidence.source_node_index,
                // `그다음` / `Then,` is the visible dependency marker on the
                // successor item; its immediate predecessor is verified from
                // the adjacent ordered item above.
                source_surface: successor_evidence.source_surface.clone(),
            });
        }
    }
    claims.extend(recovered_dependencies);

    let recovered_claim_ids = claims
        .iter()
        .map(|claim| claim.proposition_id.clone())
        .collect::<Vec<_>>();
    let mut interpretation = DocumentSemanticInterpretationIR {
        schema: DOCUMENT_SEMANTIC_INTERPRETATION_SCHEMA.into(),
        source_response_sha256: response.semantic_sha256.clone(),
        output_language,
        claims,
        recovered_claim_ids,
        unsupported_surfaces,
        ambiguous_surface_claims,
        semantic_authority: false,
        interpretation_sha256: String::new(),
    };
    interpretation.interpretation_sha256 = document_semantic_interpretation_sha256(&interpretation);
    if interpretation.validate(response) {
        return Ok(interpretation);
    }
    let missing = response
        .claims
        .iter()
        .filter(|claim| !interpretation.recovered_claim_ids.contains(&claim.proposition_id))
        .map(|claim| claim.proposition_id.as_str())
        .collect::<Vec<_>>();
    Err(format!(
        "DOCUMENT_SEMANTIC_INTERPRETATION_FAILED:RECOVERED={}:EXPECTED={}:AMBIGUOUS={}:UNSUPPORTED={}:MISSING={}",
        interpretation.recovered_claim_ids.len(),
        response.claims.len(),
        interpretation.ambiguous_surface_claims,
        interpretation.unsupported_surfaces.len(),
        missing.join(",")
    ))
}

pub fn document_response_plan_sha256(plan: &DocumentResponsePlanIR) -> String {
    let mut canonical = plan.clone();
    canonical.plan_sha256.clear();
    sha256_json(&canonical)
}

pub fn document_surface_structure_sha256(structure: &DocumentSurfaceStructureIR) -> String {
    let mut canonical = structure.clone();
    canonical.structure_sha256.clear();
    sha256_json(&canonical)
}

pub fn document_semantic_interpretation_sha256(
    interpretation: &DocumentSemanticInterpretationIR,
) -> String {
    let mut canonical = interpretation.clone();
    canonical.interpretation_sha256.clear();
    sha256_json(&canonical)
}

pub fn document_response_output_sha256(output: &DocumentResponseOutputIR) -> String {
    let mut canonical = output.clone();
    canonical.output_sha256.clear();
    sha256_json(&canonical)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSurfaceClaim {
    subject: ApprovedLexicalNodeIR,
    relation: ApprovedRelationTypeIR,
    value: ApprovedOpenValueIR,
    polarity: bool,
}

enum SemanticParseResult {
    Parsed(Vec<(String, ParsedSurfaceClaim)>),
    Unsupported,
    Ambiguous,
}

const ALL_RELATIONS: [ApprovedRelationTypeIR; 31] = [
    ApprovedRelationTypeIR::Status,
    ApprovedRelationTypeIR::Time,
    ApprovedRelationTypeIR::Cancelled,
    ApprovedRelationTypeIR::EarlierThan,
    ApprovedRelationTypeIR::RoomAvailable,
    ApprovedRelationTypeIR::Location,
    ApprovedRelationTypeIR::Registration,
    ApprovedRelationTypeIR::Entry,
    ApprovedRelationTypeIR::Confirmed,
    ApprovedRelationTypeIR::Approved,
    ApprovedRelationTypeIR::Capacity,
    ApprovedRelationTypeIR::Duration,
    ApprovedRelationTypeIR::Name,
    ApprovedRelationTypeIR::Count,
    ApprovedRelationTypeIR::Date,
    ApprovedRelationTypeIR::Quantity,
    ApprovedRelationTypeIR::Cause,
    ApprovedRelationTypeIR::Impact,
    ApprovedRelationTypeIR::Action,
    ApprovedRelationTypeIR::Owner,
    ApprovedRelationTypeIR::Deadline,
    ApprovedRelationTypeIR::Agent,
    ApprovedRelationTypeIR::Patient,
    ApprovedRelationTypeIR::Theme,
    ApprovedRelationTypeIR::Source,
    ApprovedRelationTypeIR::Destination,
    ApprovedRelationTypeIR::Instrument,
    ApprovedRelationTypeIR::Manner,
    ApprovedRelationTypeIR::Target,
    ApprovedRelationTypeIR::InitialState,
    ApprovedRelationTypeIR::ResultState,
];

const MAX_SEMANTIC_PARSE_ATTEMPTS: usize = 4_096;

fn parse_semantic_sentence_sequence(
    surface: &str,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> SemanticParseResult {
    let mut solutions = Vec::new();
    for expanded in expand_surface_realizations(surface.trim(), response, language) {
        let mut attempts = 0usize;
        let mut expanded_solutions = Vec::new();
        parse_sentence_suffix(
            &expanded,
            response,
            language,
            &mut attempts,
            &mut Vec::new(),
            &mut expanded_solutions,
        );
        for mut solution in expanded_solutions {
            if expanded != surface.trim() {
                for (source, _) in &mut solution {
                    *source = surface.trim().to_string();
                }
            }
            if !solutions.contains(&solution) {
                solutions.push(solution);
            }
            if solutions.len() >= 2 {
                break;
            }
        }
        if solutions.len() >= 2 {
            break;
        }
    }
    match solutions.len() {
        0 => SemanticParseResult::Unsupported,
        1 => SemanticParseResult::Parsed(solutions.pop().expect("one solution")),
        _ => SemanticParseResult::Ambiguous,
    }
}

fn expand_surface_realizations(
    surface: &str,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> Vec<String> {
    let mut candidates = vec![surface.to_string()];
    if let Some(expanded) = expand_event_predicates(surface, response, language) {
        candidates.push(expanded);
    }
    let mut frontier = candidates.clone();
    for _ in 0..MAX_FUSED_CLAUSES.saturating_sub(1) {
        let mut next = Vec::new();
        for candidate in frontier {
            let expansions = match language {
                LanguageCodeIR::Korean => expand_one_korean_fusion(&candidate),
                LanguageCodeIR::English => expand_one_english_fusion(&candidate),
                _ => Vec::new(),
            };
            for expanded in expansions {
                if !candidates.contains(&expanded) {
                    candidates.push(expanded.clone());
                    next.push(expanded);
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    candidates
}

fn expand_event_predicates(
    surface: &str,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> Option<String> {
    let section_specs = plan_document_sections(response);
    let sections = section_specs
        .into_iter()
        .enumerate()
        .map(|(section_index, section)| DocumentResponseSectionIR {
            section_index,
            role: section.role,
            claim_ids: section.claim_ids,
            block_indices: Vec::new(),
            predecessor_section_indices: (section_index > 0)
                .then(|| section_index - 1)
                .into_iter()
                .collect(),
            semantic_authority: false,
        })
        .collect::<Vec<_>>();
    let rhetorical_units = plan_rhetorical_units(response);
    let predicates = plan_event_predicates(response, &sections, &rhetorical_units);
    let mut expanded = surface.to_string();
    let mut replaced = false;
    for predicate in predicates {
        let claims = event_predicate_claims(&predicate, response)?;
        let event_surface = realize_event_predicate(&predicate, &claims, response, language)?;
        // A complete typed Action→event clause is already parsed as one
        // authoritative semantic bundle. Expanding it into independent role
        // assertions creates a second, weaker parse of the same sentence and
        // turns an otherwise exact inverse into ambiguity.
        let linked_action = response.claims.iter().any(|claim| {
            typed_action_event_node_id(claim, response)
                .is_some_and(|node_id| node_id == predicate.subject_node_id)
        });
        if linked_action && expanded.contains(&event_surface) {
            continue;
        }
        if expanded.contains(&event_surface) {
            let finite = claims
                .iter()
                .map(|claim| realize_claim(claim, response, language))
                .collect::<Vec<_>>()
                .join(" ");
            expanded = expanded.replacen(&event_surface, &finite, 1);
            replaced = true;
        }
    }
    replaced.then_some(expanded)
}

fn expand_one_korean_fusion(surface: &str) -> Vec<String> {
    const INVERSES: &[(&str, &[&str])] = &[
        (
            "되지 않았고",
            &["되지 않았습니다", "되지 않았어요", "되지 않았어"],
        ),
        (
            "할 수 없고",
            &["할 수 없습니다", "할 수 없어요", "할 수 없어"],
        ),
        (
            "할 수 있고",
            &["할 수 있습니다", "할 수 있어요", "할 수 있어"],
        ),
        ("변경됐고", &["변경됐습니다", "변경됐어요"]),
        ("바뀌었고", &["바뀌었어요", "바뀌었어"]),
        ("아니고", &["아닙니다", "아니에요", "아니야"]),
        ("됐고", &["됐습니다", "됐어요", "됐어"]),
        ("이고", &["입니다", "이에요", "예요", "이야", "야"]),
        ("맞고", &["맞습니다", "맞아요", "맞아"]),
    ];
    for delimiter in delimiters_outside_text(surface, ", ") {
        let left = &surface[..delimiter];
        for &(coordinated, finite_endings) in INVERSES {
            let Some(stem) = left.strip_suffix(coordinated) else {
                continue;
            };
            return finite_endings
                .iter()
                .map(|finite| {
                    format!(
                        "{stem}{finite}. 또한, {}",
                        &surface[delimiter + ", ".len()..]
                    )
                })
                .collect();
        }
    }
    Vec::new()
}

fn expand_one_english_fusion(surface: &str) -> Vec<String> {
    delimiters_outside_text(surface, ", and ")
        .into_iter()
        .next()
        .map(|delimiter| {
            vec![format!(
                "{}. Also, {}",
                &surface[..delimiter],
                &surface[delimiter + ", and ".len()..]
            )]
        })
        .unwrap_or_default()
}

fn delimiters_outside_text(surface: &str, delimiter: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in surface.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if character == '“' {
            quoted = true;
            continue;
        }
        if character == '”' {
            quoted = false;
            continue;
        }
        if !quoted && surface[index..].starts_with(delimiter) {
            positions.push(index);
        }
    }
    positions
}

fn parse_sentence_suffix(
    remaining: &str,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
    attempts: &mut usize,
    current: &mut Vec<(String, ParsedSurfaceClaim)>,
    solutions: &mut Vec<Vec<(String, ParsedSurfaceClaim)>>,
) {
    let remaining = remaining.trim_start();
    if remaining.is_empty() {
        solutions.push(current.clone());
        return;
    }
    if solutions.len() >= 2 || *attempts >= MAX_SEMANTIC_PARSE_ATTEMPTS {
        return;
    }
    for (offset, character) in remaining.char_indices() {
        if !matches!(character, '.' | '?' | '!' | '。' | '？' | '！') {
            continue;
        }
        *attempts += 1;
        let end = offset + character.len_utf8();
        let sentence = remaining[..end].trim();
        if allowed_speech_act_marker(sentence, response) {
            parse_sentence_suffix(
                &remaining[end..],
                response,
                language,
                attempts,
                current,
                solutions,
            );
            if solutions.len() >= 2 || *attempts >= MAX_SEMANTIC_PARSE_ATTEMPTS {
                return;
            }
            continue;
        }
        let context_subject = current.last().map(|(_, fact)| &fact.subject);
        let candidates = parse_one_semantic_sentence(sentence, response, language, context_subject);
        for candidate in candidates {
            let checkpoint = current.len();
            current.extend(
                candidate
                    .into_iter()
                    .map(|fact| (sentence.to_string(), fact)),
            );
            parse_sentence_suffix(
                &remaining[end..],
                response,
                language,
                attempts,
                current,
                solutions,
            );
            current.truncate(checkpoint);
            if solutions.len() >= 2 || *attempts >= MAX_SEMANTIC_PARSE_ATTEMPTS {
                return;
            }
        }
    }
}

fn parse_one_semantic_sentence(
    sentence: &str,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
    context_subject: Option<&ApprovedLexicalNodeIR>,
) -> Vec<Vec<ParsedSurfaceClaim>> {
    let body = sentence
        .strip_suffix('.')
        .or_else(|| sentence.strip_suffix('?'))
        .or_else(|| sentence.strip_suffix('!'))
        .or_else(|| sentence.strip_suffix('。'))
        .or_else(|| sentence.strip_suffix('？'))
        .or_else(|| sentence.strip_suffix('！'))
        .unwrap_or(sentence);
    if body.trim().is_empty() {
        return Vec::new();
    }
    // A direct typed event is the canonical surface of its linked Action frame.
    // Resolve it before the generic Korean parser so a partial role parse
    // cannot make the complete source-authoritative frame ambiguous.
    let typed_action_event = parse_typed_action_event_sentence(body, response, language);
    if !typed_action_event.is_empty() {
        return typed_action_event;
    }
    let mut parsed = match language {
        LanguageCodeIR::Korean => parse_korean_semantic_sentence(body, response),
        LanguageCodeIR::English => parse_english_semantic_sentence(body, response),
        _ => Vec::new(),
    }
    .into_iter()
    .map(|fact| vec![fact])
    .collect::<Vec<_>>();
    // Scoped requests and commitments leave surrounding facts asserted. Their
    // surface is therefore generated with an informational speech-act view;
    // mirror that exact typed routing during inverse parsing rather than
    // trying to recognize a Korean contextual sentence by text pattern.
    if let Some(contextual_response) = response.claims.iter().find_map(|claim| {
        action_context_surface_response(claim, response)
    }) {
        parsed.extend(
            match language {
                LanguageCodeIR::Korean => {
                    parse_korean_semantic_sentence(body, &contextual_response)
                }
                LanguageCodeIR::English => {
                    parse_english_semantic_sentence(body, &contextual_response)
                }
                _ => Vec::new(),
            }
            .into_iter()
            .map(|fact| vec![fact]),
        );
    }
    if let Some(rhetorical_body) = strip_allowed_discourse_prefix(body, response, language) {
        parsed.extend(
            match language {
                LanguageCodeIR::Korean => parse_korean_semantic_sentence(&rhetorical_body, response),
                LanguageCodeIR::English => parse_english_semantic_sentence(&rhetorical_body, response),
                _ => Vec::new(),
            }
            .into_iter()
            .map(|fact| vec![fact]),
        );
    }
    if let Some(subject) = context_subject {
        parsed.extend(
            parse_contextual_followup(body, response, language, subject)
                .into_iter()
                .map(|fact| vec![fact]),
        );
    }
    parsed.dedup();
    parsed
}

fn parse_typed_action_event_sentence(
    body: &str,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> Vec<Vec<ParsedSurfaceClaim>> {
    let Ok(plan) = build_document_response_plan(response, language) else {
        return Vec::new();
    };
    response
        .claims
        .iter()
        .filter(|claim| claim.relation == ApprovedRelationTypeIR::Action && claim.polarity)
        .filter_map(|action| {
            let predicate = action_linked_event_predicate(action, &plan, response)?;
            let arguments = event_predicate_claims(predicate, response)?;
            let event_surface = realize_event_predicate(predicate, &arguments, response, language)?;
            let candidate = match language {
                // A complete typed event already realizes the Action relation:
                // adding its action-label as a Korean nominal wrapper produces
                // repetitive report prose without adding a recoverable fact.
                // The source-authoritative Action→event binding is recovered
                // below from this exact approved predicate, never from surface
                // words in the action label.
                LanguageCodeIR::Korean => korean_typed_action_event_clause(&event_surface)?,
                LanguageCodeIR::English => format!(
                    "For {}, {}",
                    action.subject.canonical_lexical_label.trim(),
                    event_surface.trim()
                ),
                _ => return None,
            };
            let candidate = procedure_sequence_marker(action, response, language)
                .map(|marker| format!("{marker} {candidate}"))
                .unwrap_or(candidate);
            (candidate.trim_end_matches('.').trim() == body.trim()).then(|| {
                let mut facts = Vec::with_capacity(arguments.len() + 1);
                for claim in std::iter::once(action).chain(arguments.into_iter()) {
                    let (value, polarity) = effective_claim_value(claim, response);
                    facts.push(ParsedSurfaceClaim {
                        subject: claim.subject.clone(),
                        relation: claim.relation,
                        value,
                        polarity,
                    });
                }
                facts
            })
        })
        .collect()
}

fn parse_contextual_followup(
    body: &str,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
    subject: &ApprovedLexicalNodeIR,
) -> Vec<ParsedSurfaceClaim> {
    let Some(followup) = strip_allowed_discourse_prefix(body, response, language) else {
        return Vec::new();
    };
    match language {
        LanguageCodeIR::Korean => {
            let topic = object_particle(&subject.canonical_lexical_label, "은", "는").0;
            let candidates = [
                format!("{} {followup}", subject.canonical_lexical_label),
                format!("{}{topic} {followup}", subject.canonical_lexical_label),
            ];
            candidates
                .iter()
                .flat_map(|candidate| parse_korean_semantic_sentence(candidate, response))
                .collect()
        }
        LanguageCodeIR::English => {
            let mut parsed = Vec::new();
            if let Some(predicate) = followup.strip_prefix("it ") {
                parsed.extend(parse_english_semantic_sentence(
                    &format!("{} {predicate}", subject.canonical_lexical_label),
                    response,
                ));
            }
            for relation in ALL_RELATIONS {
                let relation_surface = relation_label(relation, LanguageCodeIR::English);
                let prefix = format!("its {relation_surface} ");
                if let Some(predicate) = followup.strip_prefix(&prefix) {
                    parsed.extend(parse_english_semantic_sentence(
                        &format!(
                            "The {relation_surface} of {} {predicate}",
                            subject.canonical_lexical_label
                        ),
                        response,
                    ));
                }
            }
            parsed
        }
        _ => Vec::new(),
    }
}

fn strip_allowed_discourse_prefix(
    body: &str,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> Option<String> {
    let remainder = match language {
        LanguageCodeIR::Korean => body
            .strip_prefix("또한, ")
            .or_else(|| {
                (response.discourse_relation == ApprovedDiscourseRelationIR::Correction)
                    .then(|| body.strip_prefix("정정된 내용은 다음과 같습니다. "))
                    .flatten()
            })
            .or_else(|| {
                (response.discourse_relation == ApprovedDiscourseRelationIR::Cause)
                    .then(|| body.strip_prefix("따라서, "))
                    .flatten()
            })
            .or_else(|| {
                (response.discourse_relation == ApprovedDiscourseRelationIR::Explanation)
                    .then(|| body.strip_prefix("정리하면, "))
                    .flatten()
            })
            .or_else(|| {
                (response.discourse_relation == ApprovedDiscourseRelationIR::Condition)
                    .then(|| body.strip_prefix("이 조건이 충족되면, "))
                    .flatten()
            })
            .or_else(|| {
                (response.discourse_relation == ApprovedDiscourseRelationIR::Simultaneous)
                    .then(|| body.strip_prefix("동시에, "))
                    .flatten()
            }),
        LanguageCodeIR::English => body
            .strip_prefix("Also, ")
            .or_else(|| {
                (response.discourse_relation == ApprovedDiscourseRelationIR::Correction)
                    .then(|| body.strip_prefix("Correction: "))
                    .flatten()
            })
            .or_else(|| {
                (response.discourse_relation == ApprovedDiscourseRelationIR::Cause)
                    .then(|| body.strip_prefix("Therefore, "))
                    .flatten()
            })
            .or_else(|| {
                (response.discourse_relation == ApprovedDiscourseRelationIR::Explanation)
                    .then(|| body.strip_prefix("In summary, "))
                    .flatten()
            })
            .or_else(|| {
                (response.discourse_relation == ApprovedDiscourseRelationIR::Condition)
                    .then(|| body.strip_prefix("If that condition is met, "))
                    .flatten()
            })
            .or_else(|| {
                (response.discourse_relation == ApprovedDiscourseRelationIR::Simultaneous)
                    .then(|| body.strip_prefix("At the same time, "))
                    .flatten()
            }),
        _ => None,
    }?;
    if language == LanguageCodeIR::English {
        Some(
            remainder
                .strip_prefix("the ")
                .map(|rest| format!("The {rest}"))
                .unwrap_or_else(|| remainder.to_string()),
        )
    } else {
        Some(remainder.to_string())
    }
}

fn parse_korean_semantic_sentence(
    body: &str,
    response: &ApprovedCompositionalResponseIR,
) -> Vec<ParsedSurfaceClaim> {
    let mut parsed = Vec::new();
    let subjects = response_subjects(response);
    let speech_act_endings = korean_speech_act_endings(response);

    // A typed Status frame is canonical structure.  Recover the original
    // approved claim only when its complete frame-specific Korean realization
    // matches; no predicate or threshold is inferred from arbitrary text.
    for claim in response
        .claims
        .iter()
        .filter(|claim| claim.status_frame.is_some())
    {
        let matched = [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
        ]
        .into_iter()
        .filter_map(|register| {
            korean_typed_status_frame_clause(claim, response.speech_act, register)
        })
        .any(|candidate| candidate == body);
        if matched {
            push_parsed(
                &mut parsed,
                claim.subject.clone(),
                claim.relation,
                claim.value.clone(),
                claim.polarity,
            );
        }
    }

    // Boolean properties also need a speech-act-aware predicate frame.  The
    // same approved proposition is a state predicate in INFORM/QUERY, an
    // action request in REQUEST, and a commitment in PROMISE.  Match only the
    // closed relation registry below; unknown boolean relations continue to
    // use the conservative relational fallback.
    for claim in &response.claims {
        let ApprovedOpenValueIR::Boolean(_) = claim.value else {
            continue;
        };
        let (ApprovedOpenValueIR::Boolean(asserted), true) = effective_claim_value(claim, response)
        else {
            continue;
        };
        let matched = [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
        ]
        .into_iter()
        .filter_map(|register| {
            korean_boolean_speech_act_clause(
                claim.subject.canonical_lexical_label.trim(),
                claim.relation,
                asserted,
                response.speech_act,
                register,
            )
        })
        .any(|candidate| candidate == body);
        if matched {
            push_parsed(
                &mut parsed,
                claim.subject.clone(),
                claim.relation,
                ApprovedOpenValueIR::Boolean(asserted),
                true,
            );
        }
    }

    // COUNT is a property of the subject itself in ordinary Korean.  Avoid
    // nominal stacks such as "출고 수량 개수" while keeping the typed COUNT
    // relation recoverable through the closed response context.
    for claim in &response.claims {
        let (amount, classifier) = match &claim.value {
            ApprovedOpenValueIR::Integer(amount) => (*amount, "개"),
            ApprovedOpenValueIR::Quantity { amount, unit } => {
                (*amount, unit.canonical_lexical_label.as_str())
            }
            _ => continue,
        };
        if claim.relation != ApprovedRelationTypeIR::Count
            || !claim.polarity
            || response.operation == ApprovedOperationIR::Negate
        {
            continue;
        }
        let matched = [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
        ]
        .into_iter()
        .map(|register| {
            korean_count_speech_act_clause(
                claim.subject.canonical_lexical_label.trim(),
                amount,
                classifier,
                response.speech_act,
                response.operation,
                register,
            )
        })
        .any(|candidate| candidate == body);
        if matched {
            push_parsed(
                &mut parsed,
                claim.subject.clone(),
                claim.relation,
                claim.value.clone(),
                true,
            );
        }
    }

    // Temporal ordering is a relation between two event nodes, not a nominal
    // value such as "선행 관계는 수업".  Recover the typed relation from the
    // same speech-act-aware construction used by realization.
    for claim in &response.claims {
        if claim.relation != ApprovedRelationTypeIR::EarlierThan {
            continue;
        }
        let ApprovedOpenValueIR::Lexical(reference) = &claim.value else {
            continue;
        };
        let holds = claim.polarity && response.operation != ApprovedOperationIR::Negate;
        let matched = [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
        ]
        .into_iter()
        .map(|register| {
            korean_earlier_than_speech_act_clause(
                claim.subject.canonical_lexical_label.trim(),
                reference.canonical_lexical_label.trim(),
                holds,
                response.speech_act,
                register,
            )
        })
        .any(|candidate| candidate == body);
        if matched {
            push_parsed(
                &mut parsed,
                claim.subject.clone(),
                claim.relation,
                claim.value.clone(),
                holds,
            );
        }
    }

    // Some typed lexical states are verbal predicates in Korean.  Treating
    // every lexical value as a copular noun produced grammatical-looking but
    // unusable forms such as "상태는 준비입니까" and "완료로 하겠습니다".
    // This codec registry is keyed by the approved relation and canonical
    // lexical value, never by a whole sentence.  It only recovers a claim when
    // the complete surface construction matches the approved speech act.
    for claim in &response.claims {
        let ApprovedOpenValueIR::Lexical(value) = &claim.value else {
            continue;
        };
        let subject = claim.subject.canonical_lexical_label.trim();
        let subject_topic = format!("{subject}{}", object_particle(subject, "은", "는").0);
        let subject_object = format!("{subject}{}", object_particle(subject, "을", "를").0);
        let candidates: &[String] = match (claim.relation, value.canonical_lexical_label.as_str()) {
            (ApprovedRelationTypeIR::Status, "준비") => match response.speech_act {
                ApprovedSpeechActIR::Query => &[
                    format!("{subject_topic} 준비됐습니까"),
                    format!("{subject_topic} 준비됐나요"),
                    format!("{subject_topic} 준비됐어"),
                ],
                ApprovedSpeechActIR::Request => &[
                    format!("{subject_object} 준비해 주십시오"),
                    format!("{subject_object} 준비해 주세요"),
                    format!("{subject_object} 준비해주세요"),
                    format!("{subject_object} 준비해 줘"),
                ],
                ApprovedSpeechActIR::Promise => &[
                    format!("{subject_object} 준비하겠습니다"),
                    format!("{subject_object} 준비할게요"),
                    format!("{subject_object} 준비할게"),
                ],
                _ => &[
                    format!("{subject_topic} 준비됐습니다"),
                    format!("{subject_topic} 준비됐어요"),
                    format!("{subject_topic} 준비됐어"),
                ],
            },
            (ApprovedRelationTypeIR::Registration, "완료") => match response.speech_act {
                ApprovedSpeechActIR::Query => &[
                    format!("{subject} 등록이 완료됐습니까"),
                    format!("{subject} 등록이 완료됐나요"),
                    format!("{subject} 등록이 완료됐어"),
                ],
                ApprovedSpeechActIR::Request => &[
                    format!("{subject} 등록을 완료해 주십시오"),
                    format!("{subject} 등록을 완료해 주세요"),
                    format!("{subject} 등록을 완료해주세요"),
                    format!("{subject} 등록을 완료해 줘"),
                ],
                ApprovedSpeechActIR::Promise => &[
                    format!("{subject} 등록을 완료하겠습니다"),
                    format!("{subject} 등록을 완료할게요"),
                    format!("{subject} 등록을 완료할게"),
                ],
                _ => &[
                    format!("{subject} 등록이 완료됐습니다"),
                    format!("{subject} 등록이 완료됐어요"),
                    format!("{subject} 등록이 완료됐어"),
                ],
            },
            _ => &[],
        };
        if candidates.iter().any(|candidate| candidate == body) {
            push_parsed(
                &mut parsed,
                claim.subject.clone(),
                claim.relation,
                claim.value.clone(),
                true,
            );
        }
    }
    for claim in &response.claims {
        if claim.relation != ApprovedRelationTypeIR::Status || !claim.polarity {
            continue;
        }
        let displayed = display_korean_claim_value(claim, response.style.register);
        let matched = [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
        ]
        .into_iter()
        .filter_map(|register| {
            korean_lexical_status_predicate_clause(
                claim.subject.canonical_lexical_label.trim(),
                &claim.value,
                response.speech_act,
                register,
            )
            .or_else(|| {
                korean_progress_status_clause(
                    claim.subject.canonical_lexical_label.trim(),
                    &claim.value,
                    response.speech_act,
                    register,
                )
            })
            .or_else(|| {
                korean_status_assignment_clause(
                    claim.subject.canonical_lexical_label.trim(),
                    &claim.value,
                    &displayed,
                    response.speech_act,
                    register,
                )
            })
        })
        .any(|candidate| candidate == body);
        if matched {
            push_parsed(
                &mut parsed,
                claim.subject.clone(),
                claim.relation,
                claim.value.clone(),
                true,
            );
        }
    }
    for claim in &response.claims {
        if claim.relation != ApprovedRelationTypeIR::Impact || !claim.polarity {
            continue;
        }
        let matched = [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
        ]
        .into_iter()
        .filter_map(|register| {
            korean_impact_need_clause(
                claim.subject.canonical_lexical_label.trim(),
                &claim.value,
                response.speech_act,
                register,
            )
        })
        .any(|candidate| candidate == body);
        if matched {
            push_parsed(
                &mut parsed,
                claim.subject.clone(),
                claim.relation,
                claim.value.clone(),
                true,
            );
        }
    }
    for claim in &response.claims {
        if claim.relation != ApprovedRelationTypeIR::Action || !claim.polarity {
            continue;
        }
        let matched = [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
        ]
        .into_iter()
        .filter_map(|register| {
            korean_action_relation_clause(
                claim.subject.canonical_lexical_label.trim(),
                &claim.value,
                response.speech_act,
                register,
            )
        })
        .any(|candidate| candidate == body);
        if matched {
            push_parsed(
                &mut parsed,
                claim.subject.clone(),
                claim.relation,
                claim.value.clone(),
                true,
            );
        }
    }
    for claim in &response.claims {
        if claim.relation != ApprovedRelationTypeIR::Location || !claim.polarity {
            continue;
        }
        let matched = [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
        ]
        .into_iter()
        .filter_map(|register| {
            korean_embedded_location_clause(
                claim.subject.canonical_lexical_label.trim(),
                claim.subject.semantic_type,
                &claim.value,
                response.speech_act,
                register,
            )
        })
        .any(|candidate| candidate == body);
        if matched {
            push_parsed(
                &mut parsed,
                claim.subject.clone(),
                claim.relation,
                claim.value.clone(),
                true,
            );
        }
    }
    for claim in &response.claims {
        if matches!(claim.value, ApprovedOpenValueIR::Boolean(_))
            || claim.relation == ApprovedRelationTypeIR::EarlierThan
        {
            continue;
        }
        let (_, polarity) = effective_claim_value(claim, response);
        if polarity {
            continue;
        }
        let displayed = display_korean_claim_value(claim, response.style.register);
        if let ApprovedOpenValueIR::Lexical(value) = &claim.value {
            let typed_state_matched = [
                LanguageRegisterIR::Formal,
                LanguageRegisterIR::Neutral,
                LanguageRegisterIR::Informal,
            ]
            .into_iter()
            .filter_map(|register| {
                korean_negative_typed_state_speech_act_clause(
                    claim.subject.canonical_lexical_label.trim(),
                    claim.relation,
                    value.canonical_lexical_label.as_str(),
                    response.speech_act,
                    register,
                )
            })
            .any(|candidate| candidate == body);
            if typed_state_matched {
                push_parsed(
                    &mut parsed,
                    claim.subject.clone(),
                    claim.relation,
                    claim.value.clone(),
                    false,
                );
                continue;
            }
        }
        let matched = [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
        ]
        .into_iter()
        .map(|register| {
            korean_negative_value_speech_act_clause(
                claim.subject.canonical_lexical_label.trim(),
                claim.relation,
                &claim.value,
                &displayed,
                response.speech_act,
                register,
            )
        })
        .any(|candidate| candidate == body);
        if matched {
            push_parsed(
                &mut parsed,
                claim.subject.clone(),
                claim.relation,
                claim.value.clone(),
                false,
            );
        }
    }
    for subject in &subjects {
        let subject_topic = object_particle(&subject.canonical_lexical_label, "은", "는").0;
        let prefix = format!("{}{subject_topic} ", subject.canonical_lexical_label);
        if let Some(predicate) = body.strip_prefix(&prefix) {
            for (relation, asserted, surface) in korean_boolean_predicates() {
                if predicate == surface {
                    push_parsed(
                        &mut parsed,
                        subject.clone(),
                        relation,
                        ApprovedOpenValueIR::Boolean(asserted),
                        true,
                    );
                }
            }
            // A single approved STATUS claim may omit the repeated relation
            // label (for example, "회의는 계획된 상태입니다").  The response IR
            // still supplies the subject, relation, and value, so this is a
            // generic inverse grammar alternative rather than a semantic guess
            // or a runtime surface template.
            let direct_status = response
                .claims
                .iter()
                .filter(|claim| {
                    claim.subject == *subject && claim.relation == ApprovedRelationTypeIR::Status
                })
                .collect::<Vec<_>>();
            if direct_status.len() == 1 {
                for ending in speech_act_endings {
                    if let Some(value_surface) = predicate.strip_suffix(ending) {
                        for value in parse_open_values(
                            value_surface.trim_end(),
                            response,
                            LanguageCodeIR::Korean,
                        ) {
                            push_parsed(
                                &mut parsed,
                                subject.clone(),
                                ApprovedRelationTypeIR::Status,
                                value,
                                true,
                            );
                        }
                    }
                }
            }
        }
    }

    // A Korean subject may already lexicalize the approved relation, as in
    // `초기 차수 조치` + Action or `차광막 위치` + Location.  Recover the
    // relation from the approved claim rather than requiring the redundant
    // surface `조치 조치는` / `위치 장소는`.
    for claim in &response.claims {
        if matches!(claim.value, ApprovedOpenValueIR::Boolean(_)) {
            continue;
        }
        let Some(relational_subject) = korean_subject_with_embedded_relation(
            claim.subject.canonical_lexical_label.trim(),
            claim.relation,
        ) else {
            continue;
        };
        let topic = object_particle(&relational_subject, "은", "는").0;
        let prefix = format!("{relational_subject}{topic} ");
        let Some(predicate) = body.strip_prefix(&prefix) else {
            continue;
        };
        for ending in speech_act_endings {
            if let Some(value_surface) = predicate.strip_suffix(ending) {
                for value in
                    parse_open_values(value_surface.trim_end(), response, LanguageCodeIR::Korean)
                {
                    push_parsed(
                        &mut parsed,
                        claim.subject.clone(),
                        claim.relation,
                        value,
                        true,
                    );
                }
            }
        }
    }

    if matches!(
        response.speech_act,
        ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise
    ) {
        for relation in ALL_RELATIONS {
            let relation_surface = relation_label(relation, LanguageCodeIR::Korean);
            let object = object_particle(relation_surface, "을", "를").0;
            for subject in &subjects {
                let prefix = format!(
                    "{} {relation_surface}{object} ",
                    subject.canonical_lexical_label
                );
                let Some(predicate) = body.strip_prefix(&prefix) else {
                    continue;
                };
                for ending in speech_act_endings {
                    if let Some(value_surface) = predicate.strip_suffix(ending) {
                        for value in parse_open_values(
                            value_surface.trim_end(),
                            response,
                            LanguageCodeIR::Korean,
                        ) {
                            push_parsed(&mut parsed, subject.clone(), relation, value, true);
                        }
                    }
                }
            }
        }
    }

    for relation in ALL_RELATIONS {
        let relation_surface = relation_label(relation, LanguageCodeIR::Korean);
        let topic = object_particle(relation_surface, "은", "는").0;
        for subject in &subjects {
            let prefix = format!(
                "{} {relation_surface}{topic} ",
                subject.canonical_lexical_label
            );
            let Some(predicate) = body.strip_prefix(&prefix) else {
                continue;
            };
            if let Some(asserted) = korean_truth_predicate(predicate) {
                push_parsed(
                    &mut parsed,
                    subject.clone(),
                    relation,
                    ApprovedOpenValueIR::Boolean(asserted),
                    true,
                );
                continue;
            }
            for ending in [" 변경됐습니다", " 변경됐어요", " 바뀌었어요", " 바뀌었어"]
            {
                if let Some(value_with_particle) = predicate.strip_suffix(ending) {
                    for value in
                        parse_korean_particle_value(value_with_particle, "으로", "로", response)
                    {
                        push_parsed(&mut parsed, subject.clone(), relation, value, true);
                    }
                }
            }
            for ending in [" 아닙니다", " 아니에요", " 아니야"] {
                if let Some(value_with_particle) = predicate.strip_suffix(ending) {
                    for value in
                        parse_korean_particle_value(value_with_particle, "이", "가", response)
                    {
                        push_parsed(&mut parsed, subject.clone(), relation, value, false);
                    }
                }
            }
            for ending in speech_act_endings {
                if let Some(value_surface) = predicate.strip_suffix(ending) {
                    for value in parse_open_values(
                        value_surface.trim_end(),
                        response,
                        LanguageCodeIR::Korean,
                    ) {
                        push_parsed(&mut parsed, subject.clone(), relation, value, true);
                    }
                }
            }
        }
    }

    parsed
}

fn korean_speech_act_endings(
    response: &ApprovedCompositionalResponseIR,
) -> &'static [&'static str] {
    match response.speech_act {
        ApprovedSpeechActIR::Query => {
            &["인가요", "입니까", "예요", "이에요", "입니다", "이야", "야"]
        }
        ApprovedSpeechActIR::Request => &[
            "으로 해 주세요",
            "으로 해주세요",
            "으로 해 주십시오",
            "으로 해주십시오",
            "으로 해 줘",
            "로 해 주세요",
            "로 해주세요",
            "로 해 주십시오",
            "로 해주십시오",
            "로 해 줘",
            "해 주세요",
            "해주세요",
            "해 주십시오",
            "해주십시오",
            "해 줘",
            "부탁드립니다",
            "부탁드려요",
            "입니다",
            "이에요",
            "예요",
        ],
        ApprovedSpeechActIR::Promise => &[
            "으로 하겠습니다",
            "으로 할게요",
            "으로 할게",
            "로 하겠습니다",
            "로 할게요",
            "로 할게",
            "하겠습니다",
            "할게요",
            "약속드리겠습니다",
            "약속드려요",
            "입니다",
            "이에요",
            "예요",
        ],
        ApprovedSpeechActIR::Reassure => &["입니다", "이에요", "예요", "이야", "야"],
        _ => &["입니다", "이에요", "예요", "이야", "야"],
    }
}

fn parse_english_semantic_sentence(
    body: &str,
    response: &ApprovedCompositionalResponseIR,
) -> Vec<ParsedSurfaceClaim> {
    let mut parsed = Vec::new();
    let subjects = response_subjects(response);
    for subject in &subjects {
        let prefix = format!("{} ", subject.canonical_lexical_label);
        if let Some(predicate) = body.strip_prefix(&prefix) {
            for (relation, asserted, surface) in english_boolean_predicates() {
                if predicate == surface {
                    push_parsed(
                        &mut parsed,
                        subject.clone(),
                        relation,
                        ApprovedOpenValueIR::Boolean(asserted),
                        true,
                    );
                }
            }
        }
    }

    for relation in ALL_RELATIONS {
        let relation_surface = relation_label(relation, LanguageCodeIR::English);
        for subject in &subjects {
            let prefix = format!(
                "The {relation_surface} of {} ",
                subject.canonical_lexical_label
            );
            let Some(predicate) = body.strip_prefix(&prefix) else {
                continue;
            };
            if predicate == "is true" || predicate == "is false" {
                push_parsed(
                    &mut parsed,
                    subject.clone(),
                    relation,
                    ApprovedOpenValueIR::Boolean(predicate == "is true"),
                    true,
                );
                continue;
            }
            for (prefix, polarity) in [("has changed to ", true), ("is not ", false), ("is ", true)]
            {
                if let Some(value_surface) = predicate.strip_prefix(prefix) {
                    for value in parse_open_values(value_surface, response, LanguageCodeIR::English)
                    {
                        push_parsed(&mut parsed, subject.clone(), relation, value, polarity);
                    }
                }
            }
        }
    }
    parsed
}

fn response_subjects(response: &ApprovedCompositionalResponseIR) -> Vec<ApprovedLexicalNodeIR> {
    let mut subjects = Vec::new();
    for claim in &response.claims {
        if !subjects.contains(&claim.subject) {
            subjects.push(claim.subject.clone());
        }
    }
    subjects
}

fn korean_boolean_predicates() -> Vec<(ApprovedRelationTypeIR, bool, &'static str)> {
    vec![
        (ApprovedRelationTypeIR::Cancelled, true, "취소됐습니다"),
        (ApprovedRelationTypeIR::Cancelled, true, "취소됐어요"),
        (ApprovedRelationTypeIR::Cancelled, true, "취소됐어"),
        (
            ApprovedRelationTypeIR::Cancelled,
            false,
            "취소되지 않았습니다",
        ),
        (
            ApprovedRelationTypeIR::Cancelled,
            false,
            "취소되지 않았어요",
        ),
        (ApprovedRelationTypeIR::Cancelled, false, "취소되지 않았어"),
        (
            ApprovedRelationTypeIR::RoomAvailable,
            true,
            "사용할 수 있습니다",
        ),
        (
            ApprovedRelationTypeIR::RoomAvailable,
            true,
            "사용할 수 있어요",
        ),
        (
            ApprovedRelationTypeIR::RoomAvailable,
            true,
            "사용할 수 있어",
        ),
        (
            ApprovedRelationTypeIR::RoomAvailable,
            false,
            "사용할 수 없습니다",
        ),
        (
            ApprovedRelationTypeIR::RoomAvailable,
            false,
            "사용할 수 없어요",
        ),
        (
            ApprovedRelationTypeIR::RoomAvailable,
            false,
            "사용할 수 없어",
        ),
        (ApprovedRelationTypeIR::Registration, true, "등록됐습니다"),
        (ApprovedRelationTypeIR::Registration, true, "등록됐어요"),
        (ApprovedRelationTypeIR::Registration, true, "등록됐어"),
        (
            ApprovedRelationTypeIR::Registration,
            false,
            "등록되지 않았습니다",
        ),
        (
            ApprovedRelationTypeIR::Registration,
            false,
            "등록되지 않았어요",
        ),
        (
            ApprovedRelationTypeIR::Registration,
            false,
            "등록되지 않았어",
        ),
        (ApprovedRelationTypeIR::Entry, true, "입장할 수 있습니다"),
        (ApprovedRelationTypeIR::Entry, true, "입장할 수 있어요"),
        (ApprovedRelationTypeIR::Entry, true, "입장할 수 있어"),
        (ApprovedRelationTypeIR::Entry, false, "입장할 수 없습니다"),
        (ApprovedRelationTypeIR::Entry, false, "입장할 수 없어요"),
        (ApprovedRelationTypeIR::Entry, false, "입장할 수 없어"),
        (ApprovedRelationTypeIR::Confirmed, true, "확정됐습니다"),
        (ApprovedRelationTypeIR::Confirmed, true, "확정됐어요"),
        (ApprovedRelationTypeIR::Confirmed, true, "확정됐어"),
        (
            ApprovedRelationTypeIR::Confirmed,
            false,
            "확정되지 않았습니다",
        ),
        (
            ApprovedRelationTypeIR::Confirmed,
            false,
            "확정되지 않았어요",
        ),
        (ApprovedRelationTypeIR::Confirmed, false, "확정되지 않았어"),
        (ApprovedRelationTypeIR::Approved, true, "승인됐습니다"),
        (ApprovedRelationTypeIR::Approved, true, "승인됐어요"),
        (ApprovedRelationTypeIR::Approved, true, "승인됐어"),
        (
            ApprovedRelationTypeIR::Approved,
            false,
            "승인되지 않았습니다",
        ),
        (ApprovedRelationTypeIR::Approved, false, "승인되지 않았어요"),
        (ApprovedRelationTypeIR::Approved, false, "승인되지 않았어"),
    ]
}

fn korean_boolean_speech_act_clause(
    subject: &str,
    relation: ApprovedRelationTypeIR,
    asserted: bool,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> Option<String> {
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let display_subject = subject;
    let topic = format!(
        "{display_subject}{}",
        object_particle(display_subject, "은", "는").0
    );
    let object = format!("{subject}{}", object_particle(subject, "을", "를").0);
    let locative = format!("{subject}에");

    let indicative = match (relation, asserted, formal, casual) {
        (ApprovedRelationTypeIR::Cancelled, true, true, _) => format!("{topic} 취소됐습니다"),
        (ApprovedRelationTypeIR::Cancelled, true, false, false) => format!("{topic} 취소됐어요"),
        (ApprovedRelationTypeIR::Cancelled, true, false, true) => format!("{topic} 취소됐어"),
        (ApprovedRelationTypeIR::Cancelled, false, true, _) => {
            format!("{topic} 취소되지 않았습니다")
        }
        (ApprovedRelationTypeIR::Cancelled, false, false, false) => {
            format!("{topic} 취소되지 않았어요")
        }
        (ApprovedRelationTypeIR::Cancelled, false, false, true) => {
            format!("{topic} 취소되지 않았어")
        }
        (ApprovedRelationTypeIR::RoomAvailable, true, true, _) => {
            format!("{object} 사용할 수 있습니다")
        }
        (ApprovedRelationTypeIR::RoomAvailable, true, false, false) => {
            format!("{object} 사용할 수 있어요")
        }
        (ApprovedRelationTypeIR::RoomAvailable, true, false, true) => {
            format!("{object} 사용할 수 있어")
        }
        (ApprovedRelationTypeIR::RoomAvailable, false, true, _) => {
            format!("{object} 사용할 수 없습니다")
        }
        (ApprovedRelationTypeIR::RoomAvailable, false, false, false) => {
            format!("{object} 사용할 수 없어요")
        }
        (ApprovedRelationTypeIR::RoomAvailable, false, false, true) => {
            format!("{object} 사용할 수 없어")
        }
        (ApprovedRelationTypeIR::Registration, true, true, _) => format!("{topic} 등록됐습니다"),
        (ApprovedRelationTypeIR::Registration, true, false, false) => format!("{topic} 등록됐어요"),
        (ApprovedRelationTypeIR::Registration, true, false, true) => format!("{topic} 등록됐어"),
        (ApprovedRelationTypeIR::Registration, false, true, _) => {
            format!("{topic} 등록되지 않았습니다")
        }
        (ApprovedRelationTypeIR::Registration, false, false, false) => {
            format!("{topic} 등록되지 않았어요")
        }
        (ApprovedRelationTypeIR::Registration, false, false, true) => {
            format!("{topic} 등록되지 않았어")
        }
        (ApprovedRelationTypeIR::Entry, true, true, _) => format!("{topic} 입장할 수 있습니다"),
        (ApprovedRelationTypeIR::Entry, true, false, false) => {
            format!("{topic} 입장할 수 있어요")
        }
        (ApprovedRelationTypeIR::Entry, true, false, true) => format!("{topic} 입장할 수 있어"),
        (ApprovedRelationTypeIR::Entry, false, true, _) => format!("{topic} 입장할 수 없습니다"),
        (ApprovedRelationTypeIR::Entry, false, false, false) => {
            format!("{topic} 입장할 수 없어요")
        }
        (ApprovedRelationTypeIR::Entry, false, false, true) => format!("{topic} 입장할 수 없어"),
        (ApprovedRelationTypeIR::Confirmed, true, true, _) => format!("{topic} 확정됐습니다"),
        (ApprovedRelationTypeIR::Confirmed, true, false, false) => format!("{topic} 확정됐어요"),
        (ApprovedRelationTypeIR::Confirmed, true, false, true) => format!("{topic} 확정됐어"),
        (ApprovedRelationTypeIR::Confirmed, false, true, _) => {
            format!("{topic} 확정되지 않았습니다")
        }
        (ApprovedRelationTypeIR::Confirmed, false, false, false) => {
            format!("{topic} 확정되지 않았어요")
        }
        (ApprovedRelationTypeIR::Confirmed, false, false, true) => {
            format!("{topic} 확정되지 않았어")
        }
        (ApprovedRelationTypeIR::Approved, true, true, _) => format!("{topic} 승인됐습니다"),
        (ApprovedRelationTypeIR::Approved, true, false, false) => format!("{topic} 승인됐어요"),
        (ApprovedRelationTypeIR::Approved, true, false, true) => format!("{topic} 승인됐어"),
        (ApprovedRelationTypeIR::Approved, false, true, _) => {
            format!("{topic} 승인되지 않았습니다")
        }
        (ApprovedRelationTypeIR::Approved, false, false, false) => {
            format!("{topic} 승인되지 않았어요")
        }
        (ApprovedRelationTypeIR::Approved, false, false, true) => {
            format!("{topic} 승인되지 않았어")
        }
        _ => return None,
    };

    Some(match speech_act {
        ApprovedSpeechActIR::Query => match (relation, asserted, formal, casual) {
            (ApprovedRelationTypeIR::Cancelled, true, true, _) => format!("{topic} 취소됐습니까"),
            (ApprovedRelationTypeIR::Cancelled, true, false, false) => {
                format!("{topic} 취소됐나요")
            }
            (ApprovedRelationTypeIR::Cancelled, true, false, true) => format!("{topic} 취소됐어"),
            (ApprovedRelationTypeIR::Cancelled, false, true, _) => {
                format!("{topic} 취소되지 않았습니까")
            }
            (ApprovedRelationTypeIR::Cancelled, false, false, false) => {
                format!("{topic} 취소되지 않았나요")
            }
            (ApprovedRelationTypeIR::Cancelled, false, false, true) => {
                format!("{topic} 취소되지 않았어")
            }
            (ApprovedRelationTypeIR::RoomAvailable, true, true, _) => {
                format!("{object} 사용할 수 있습니까")
            }
            (ApprovedRelationTypeIR::RoomAvailable, true, false, false) => {
                format!("{object} 사용할 수 있나요")
            }
            (ApprovedRelationTypeIR::RoomAvailable, true, false, true) => {
                format!("{object} 사용할 수 있어")
            }
            (ApprovedRelationTypeIR::RoomAvailable, false, true, _) => {
                format!("{object} 사용할 수 없습니까")
            }
            (ApprovedRelationTypeIR::RoomAvailable, false, false, false) => {
                format!("{object} 사용할 수 없나요")
            }
            (ApprovedRelationTypeIR::RoomAvailable, false, false, true) => {
                format!("{object} 사용할 수 없어")
            }
            (ApprovedRelationTypeIR::Registration, true, true, _) => {
                format!("{topic} 등록됐습니까")
            }
            (ApprovedRelationTypeIR::Registration, true, false, false) => {
                format!("{topic} 등록됐나요")
            }
            (ApprovedRelationTypeIR::Registration, true, false, true) => {
                format!("{topic} 등록됐어")
            }
            (ApprovedRelationTypeIR::Registration, false, true, _) => {
                format!("{topic} 등록되지 않았습니까")
            }
            (ApprovedRelationTypeIR::Registration, false, false, false) => {
                format!("{topic} 등록되지 않았나요")
            }
            (ApprovedRelationTypeIR::Registration, false, false, true) => {
                format!("{topic} 등록되지 않았어")
            }
            (ApprovedRelationTypeIR::Entry, true, true, _) => {
                format!("{locative} 입장할 수 있습니까")
            }
            (ApprovedRelationTypeIR::Entry, true, false, false) => {
                format!("{locative} 입장할 수 있나요")
            }
            (ApprovedRelationTypeIR::Entry, true, false, true) => {
                format!("{locative} 입장할 수 있어")
            }
            (ApprovedRelationTypeIR::Entry, false, true, _) => {
                format!("{locative} 입장할 수 없습니까")
            }
            (ApprovedRelationTypeIR::Entry, false, false, false) => {
                format!("{locative} 입장할 수 없나요")
            }
            (ApprovedRelationTypeIR::Entry, false, false, true) => {
                format!("{locative} 입장할 수 없어")
            }
            (ApprovedRelationTypeIR::Confirmed, true, true, _) => format!("{topic} 확정됐습니까"),
            (ApprovedRelationTypeIR::Confirmed, true, false, false) => {
                format!("{topic} 확정됐나요")
            }
            (ApprovedRelationTypeIR::Confirmed, true, false, true) => format!("{topic} 확정됐어"),
            (ApprovedRelationTypeIR::Confirmed, false, true, _) => {
                format!("{topic} 확정되지 않았습니까")
            }
            (ApprovedRelationTypeIR::Confirmed, false, false, false) => {
                format!("{topic} 확정되지 않았나요")
            }
            (ApprovedRelationTypeIR::Confirmed, false, false, true) => {
                format!("{topic} 확정되지 않았어")
            }
            (ApprovedRelationTypeIR::Approved, true, true, _) => format!("{topic} 승인됐습니까"),
            (ApprovedRelationTypeIR::Approved, true, false, false) => {
                format!("{topic} 승인됐나요")
            }
            (ApprovedRelationTypeIR::Approved, true, false, true) => {
                format!("{topic} 승인됐어")
            }
            (ApprovedRelationTypeIR::Approved, false, true, _) => {
                format!("{topic} 승인되지 않았습니까")
            }
            (ApprovedRelationTypeIR::Approved, false, false, false) => {
                format!("{topic} 승인되지 않았나요")
            }
            (ApprovedRelationTypeIR::Approved, false, false, true) => {
                format!("{topic} 승인되지 않았어")
            }
            _ => unreachable!("indicative registry already rejected unsupported relation"),
        },
        ApprovedSpeechActIR::Request => match (relation, asserted, formal, casual) {
            (ApprovedRelationTypeIR::Cancelled, true, true, _) => {
                format!("{object} 취소해 주십시오")
            }
            (ApprovedRelationTypeIR::Cancelled, true, false, false) => {
                format!("{object} 취소해 주세요")
            }
            (ApprovedRelationTypeIR::Cancelled, true, false, true) => format!("{object} 취소해 줘"),
            (ApprovedRelationTypeIR::Cancelled, false, true, _) => {
                format!("{object} 취소하지 말아 주십시오")
            }
            (ApprovedRelationTypeIR::Cancelled, false, false, false) => {
                format!("{object} 취소하지 말아 주세요")
            }
            (ApprovedRelationTypeIR::Cancelled, false, false, true) => {
                format!("{object} 취소하지 마")
            }
            (ApprovedRelationTypeIR::RoomAvailable, value, true, _) => format!(
                "{object} 사용할 수 {} 해 주십시오",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::RoomAvailable, value, false, false) => format!(
                "{object} 사용할 수 {} 해 주세요",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::RoomAvailable, value, false, true) => format!(
                "{object} 사용할 수 {} 해 줘",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::Registration, true, true, _) => {
                format!("{object} 등록해 주십시오")
            }
            (ApprovedRelationTypeIR::Registration, true, false, false) => {
                format!("{object} 등록해 주세요")
            }
            (ApprovedRelationTypeIR::Registration, true, false, true) => {
                format!("{object} 등록해 줘")
            }
            (ApprovedRelationTypeIR::Registration, false, true, _) => {
                format!("{object} 등록하지 말아 주십시오")
            }
            (ApprovedRelationTypeIR::Registration, false, false, false) => {
                format!("{object} 등록하지 말아 주세요")
            }
            (ApprovedRelationTypeIR::Registration, false, false, true) => {
                format!("{object} 등록하지 마")
            }
            (ApprovedRelationTypeIR::Entry, value, true, _) => format!(
                "{locative} 입장할 수 {} 해 주십시오",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::Entry, value, false, false) => format!(
                "{locative} 입장할 수 {} 해 주세요",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::Entry, value, false, true) => format!(
                "{locative} 입장할 수 {} 해 줘",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::Confirmed, true, true, _) => {
                format!("{object} 확정해 주십시오")
            }
            (ApprovedRelationTypeIR::Confirmed, true, false, false) => {
                format!("{object} 확정해 주세요")
            }
            (ApprovedRelationTypeIR::Confirmed, true, false, true) => format!("{object} 확정해 줘"),
            (ApprovedRelationTypeIR::Confirmed, false, true, _) => {
                format!("{object} 확정하지 말아 주십시오")
            }
            (ApprovedRelationTypeIR::Confirmed, false, false, false) => {
                format!("{object} 확정하지 말아 주세요")
            }
            (ApprovedRelationTypeIR::Confirmed, false, false, true) => {
                format!("{object} 확정하지 마")
            }
            (ApprovedRelationTypeIR::Approved, true, true, _) => {
                format!("{object} 승인해 주십시오")
            }
            (ApprovedRelationTypeIR::Approved, true, false, false) => {
                format!("{object} 승인해 주세요")
            }
            (ApprovedRelationTypeIR::Approved, true, false, true) => {
                format!("{object} 승인해 줘")
            }
            (ApprovedRelationTypeIR::Approved, false, true, _) => {
                format!("{object} 승인하지 말아 주십시오")
            }
            (ApprovedRelationTypeIR::Approved, false, false, false) => {
                format!("{object} 승인하지 말아 주세요")
            }
            (ApprovedRelationTypeIR::Approved, false, false, true) => {
                format!("{object} 승인하지 마")
            }
            _ => unreachable!("indicative registry already rejected unsupported relation"),
        },
        ApprovedSpeechActIR::Promise => match (relation, asserted, formal, casual) {
            (ApprovedRelationTypeIR::Cancelled, true, true, _) => {
                format!("{object} 취소하겠습니다")
            }
            (ApprovedRelationTypeIR::Cancelled, true, false, false) => {
                format!("{object} 취소할게요")
            }
            (ApprovedRelationTypeIR::Cancelled, true, false, true) => format!("{object} 취소할게"),
            (ApprovedRelationTypeIR::Cancelled, false, true, _) => {
                format!("{object} 취소하지 않겠습니다")
            }
            (ApprovedRelationTypeIR::Cancelled, false, false, false) => {
                format!("{object} 취소하지 않을게요")
            }
            (ApprovedRelationTypeIR::Cancelled, false, false, true) => {
                format!("{object} 취소하지 않을게")
            }
            (ApprovedRelationTypeIR::RoomAvailable, value, true, _) => format!(
                "{object} 사용할 수 {} 하겠습니다",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::RoomAvailable, value, false, false) => format!(
                "{object} 사용할 수 {} 할게요",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::RoomAvailable, value, false, true) => format!(
                "{object} 사용할 수 {} 할게",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::Registration, true, true, _) => {
                format!("{object} 등록하겠습니다")
            }
            (ApprovedRelationTypeIR::Registration, true, false, false) => {
                format!("{object} 등록할게요")
            }
            (ApprovedRelationTypeIR::Registration, true, false, true) => {
                format!("{object} 등록할게")
            }
            (ApprovedRelationTypeIR::Registration, false, true, _) => {
                format!("{object} 등록하지 않겠습니다")
            }
            (ApprovedRelationTypeIR::Registration, false, false, false) => {
                format!("{object} 등록하지 않을게요")
            }
            (ApprovedRelationTypeIR::Registration, false, false, true) => {
                format!("{object} 등록하지 않을게")
            }
            (ApprovedRelationTypeIR::Entry, value, true, _) => format!(
                "{locative} 입장할 수 {} 하겠습니다",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::Entry, value, false, false) => format!(
                "{locative} 입장할 수 {} 할게요",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::Entry, value, false, true) => format!(
                "{locative} 입장할 수 {} 할게",
                if value { "있게" } else { "없게" }
            ),
            (ApprovedRelationTypeIR::Confirmed, true, true, _) => {
                format!("{object} 확정하겠습니다")
            }
            (ApprovedRelationTypeIR::Confirmed, true, false, false) => {
                format!("{object} 확정할게요")
            }
            (ApprovedRelationTypeIR::Confirmed, true, false, true) => format!("{object} 확정할게"),
            (ApprovedRelationTypeIR::Confirmed, false, true, _) => {
                format!("{object} 확정하지 않겠습니다")
            }
            (ApprovedRelationTypeIR::Confirmed, false, false, false) => {
                format!("{object} 확정하지 않을게요")
            }
            (ApprovedRelationTypeIR::Confirmed, false, false, true) => {
                format!("{object} 확정하지 않을게")
            }
            (ApprovedRelationTypeIR::Approved, true, true, _) => {
                format!("{object} 승인하겠습니다")
            }
            (ApprovedRelationTypeIR::Approved, true, false, false) => {
                format!("{object} 승인할게요")
            }
            (ApprovedRelationTypeIR::Approved, true, false, true) => {
                format!("{object} 승인할게")
            }
            (ApprovedRelationTypeIR::Approved, false, true, _) => {
                format!("{object} 승인하지 않겠습니다")
            }
            (ApprovedRelationTypeIR::Approved, false, false, false) => {
                format!("{object} 승인하지 않을게요")
            }
            (ApprovedRelationTypeIR::Approved, false, false, true) => {
                format!("{object} 승인하지 않을게")
            }
            _ => unreachable!("indicative registry already rejected unsupported relation"),
        },
        ApprovedSpeechActIR::Reassure | ApprovedSpeechActIR::Inform => indicative,
        _ => indicative,
    })
}

fn korean_earlier_than_speech_act_clause(
    subject: &str,
    reference: &str,
    holds: bool,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> String {
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let topic = format!("{subject}{}", object_particle(subject, "은", "는").0);
    let object = format!("{subject}{}", object_particle(subject, "을", "를").0);
    let comparison = format!("{reference}보다");

    match speech_act {
        ApprovedSpeechActIR::Query => match (holds, formal, casual) {
            (true, true, _) => format!("{topic} {comparison} 앞섭니까"),
            (true, false, false) => format!("{topic} {comparison} 앞서나요"),
            (true, false, true) => format!("{topic} {comparison} 앞서"),
            (false, true, _) => format!("{topic} {comparison} 앞서지 않습니까"),
            (false, false, false) => format!("{topic} {comparison} 앞서지 않나요"),
            (false, false, true) => format!("{topic} {comparison} 앞서지 않아"),
        },
        ApprovedSpeechActIR::Request => match (holds, formal, casual) {
            (true, true, _) => format!("{object} {comparison} 앞서 배치해 주십시오"),
            (true, false, false) => format!("{object} {comparison} 앞서 배치해 주세요"),
            (true, false, true) => format!("{object} {comparison} 앞서 배치해 줘"),
            (false, true, _) => {
                format!("{object} {comparison} 앞서 배치하지 말아 주십시오")
            }
            (false, false, false) => {
                format!("{object} {comparison} 앞서 배치하지 말아 주세요")
            }
            (false, false, true) => format!("{object} {comparison} 앞서 배치하지 마"),
        },
        ApprovedSpeechActIR::Promise => match (holds, formal, casual) {
            (true, true, _) => format!("{object} {comparison} 앞서 배치하겠습니다"),
            (true, false, false) => format!("{object} {comparison} 앞서 배치할게요"),
            (true, false, true) => format!("{object} {comparison} 앞서 배치할게"),
            (false, true, _) => format!("{object} {comparison} 앞서 배치하지 않겠습니다"),
            (false, false, false) => {
                format!("{object} {comparison} 앞서 배치하지 않을게요")
            }
            (false, false, true) => format!("{object} {comparison} 앞서 배치하지 않을게"),
        },
        _ => match (holds, formal, casual) {
            (true, true, _) => format!("{topic} {comparison} 앞섭니다"),
            (true, false, false) => format!("{topic} {comparison} 앞서요"),
            (true, false, true) => format!("{topic} {comparison} 앞서"),
            (false, true, _) => format!("{topic} {comparison} 앞서지 않습니다"),
            (false, false, false) => format!("{topic} {comparison} 앞서지 않아요"),
            (false, false, true) => format!("{topic} {comparison} 앞서지 않아"),
        },
    }
}

fn korean_status_assignment_clause(
    subject: &str,
    value: &ApprovedOpenValueIR,
    displayed_value: &str,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> Option<String> {
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let complement = directional_particle(particle_basis(value, displayed_value)).0;
    match speech_act {
        ApprovedSpeechActIR::Request => Some(format!(
            "{subject} 상태를 {displayed_value}{complement} {}",
            if formal {
                "설정해 주십시오"
            } else if casual {
                "설정해 줘"
            } else {
                "설정해 주세요"
            }
        )),
        ApprovedSpeechActIR::Promise => Some(format!(
            "{subject} 상태를 {displayed_value}{complement} {}",
            if formal {
                "설정하겠습니다"
            } else if casual {
                "설정할게"
            } else {
                "설정할게요"
            }
        )),
        _ => None,
    }
}

/// Realize a status whose predicate structure was explicitly approved in the
/// canonical claim.  The frame is deliberately small: it distinguishes a
/// configured state, an observed indicator, and an operational threshold
/// without deriving any of those meanings from a Korean label.
fn korean_typed_status_frame_clause(
    claim: &ApprovedCompositionalClaimIR,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> Option<String> {
    if claim.relation != ApprovedRelationTypeIR::Status || !claim.polarity {
        return None;
    }
    let frame = claim.status_frame.as_ref()?;
    let subject = claim.subject.canonical_lexical_label.trim();
    let topic = format!("{subject}{}", object_particle(subject, "은", "는").0);
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    match frame {
        ApprovedStatusFrameIR::Configuration {
            predicate,
            modifier,
        } => {
            let predicate = match (predicate, speech_act, formal, casual) {
                (ApprovedConfigurationStateIR::Open, ApprovedSpeechActIR::Query, true, _) => {
                    "개방되어 있습니까"
                }
                (ApprovedConfigurationStateIR::Open, ApprovedSpeechActIR::Query, false, true) => {
                    "개방되어 있어"
                }
                (ApprovedConfigurationStateIR::Open, ApprovedSpeechActIR::Query, false, false) => {
                    "개방되어 있나요"
                }
                (ApprovedConfigurationStateIR::Open, _, true, _) => "개방되어 있습니다",
                (ApprovedConfigurationStateIR::Open, _, false, true) => "개방되어 있어",
                (ApprovedConfigurationStateIR::Open, _, false, false) => "개방되어 있어요",
                (ApprovedConfigurationStateIR::Closed, _, true, _) => "폐쇄되어 있습니다",
                (ApprovedConfigurationStateIR::Closed, _, false, true) => "폐쇄되어 있어",
                (ApprovedConfigurationStateIR::Closed, _, false, false) => "폐쇄되어 있어요",
                (ApprovedConfigurationStateIR::Enabled, _, true, _) => "활성화되어 있습니다",
                (ApprovedConfigurationStateIR::Enabled, _, false, true) => "활성화되어 있어",
                (ApprovedConfigurationStateIR::Enabled, _, false, false) => "활성화되어 있어요",
                (ApprovedConfigurationStateIR::Disabled, _, true, _) => "비활성화되어 있습니다",
                (ApprovedConfigurationStateIR::Disabled, _, false, true) => "비활성화되어 있어",
                (ApprovedConfigurationStateIR::Disabled, _, false, false) => "비활성화되어 있어요",
            };
            let modifier = modifier.as_ref().map(|modifier| {
                let label = modifier.canonical_lexical_label.trim();
                format!(" {label}{}", directional_particle(label).0)
            });
            Some(format!("{topic}{} {predicate}", modifier.unwrap_or_default()))
        }
        ApprovedStatusFrameIR::Observation { evidence } => {
            let evidence = evidence
                .iter()
                .map(|node| node.canonical_lexical_label.trim())
                .collect::<Vec<_>>();
            let evidence = match evidence.as_slice() {
                [] => return None,
                [single] => (*single).to_string(),
                many => format!("{}과 {}", many[..many.len() - 1].join("과 "), many[many.len() - 1]),
            };
            let predicate = match speech_act {
                ApprovedSpeechActIR::Query if formal => "나타납니까",
                ApprovedSpeechActIR::Query if casual => "나타나",
                ApprovedSpeechActIR::Query => "나타나나요",
                _ if formal => "나타납니다",
                _ if casual => "나타나",
                _ => "나타나요",
            };
            Some(format!("{topic} {evidence}로 {predicate}"))
        }
        ApprovedStatusFrameIR::Threshold { trigger } => {
            let trigger = trigger.canonical_lexical_label.trim();
            let predicate = match speech_act {
                ApprovedSpeechActIR::Query if formal => "입니까",
                ApprovedSpeechActIR::Query if casual => "야",
                ApprovedSpeechActIR::Query => "인가요",
                _ if formal => "입니다",
                _ if casual => "야",
                _ => "예요",
            };
            Some(format!("{topic} {trigger} 시{predicate}"))
        }
        ApprovedStatusFrameIR::WithinRange { range } => {
            let range = range.canonical_lexical_label.trim();
            let predicate = match speech_act {
                ApprovedSpeechActIR::Query if formal => "있습니까",
                ApprovedSpeechActIR::Query if casual => "있어",
                ApprovedSpeechActIR::Query => "있나요",
                _ if formal => "있습니다",
                _ if casual => "있어",
                _ => "있어요",
            };
            Some(format!("{topic} {range} 안에 {predicate}"))
        }
        ApprovedStatusFrameIR::CapacitySufficient { criterion } => {
            let criterion = criterion.canonical_lexical_label.trim();
            let predicate = match speech_act {
                ApprovedSpeechActIR::Query if formal => "충족합니까",
                ApprovedSpeechActIR::Query if casual => "충족해",
                ApprovedSpeechActIR::Query => "충족하나요",
                _ if formal => "충족합니다",
                _ if casual => "충족해",
                _ => "충족해요",
            };
            Some(format!("{topic} {criterion}을 {predicate}"))
        }
        ApprovedStatusFrameIR::Verification { finding } => {
            let finding = finding.canonical_lexical_label.trim();
            let predicate = match speech_act {
                ApprovedSpeechActIR::Query if formal => "확인됐습니까",
                ApprovedSpeechActIR::Query if casual => "확인됐어",
                ApprovedSpeechActIR::Query => "확인됐나요",
                _ if formal => "확인됐습니다",
                _ if casual => "확인됐어",
                _ => "확인됐어요",
            };
            Some(format!(
                "{subject}에서 {} {predicate}",
                with_particle(finding, "이", "가")
            ))
        }
    }
}

fn korean_lexical_status_predicate_clause(
    subject: &str,
    value: &ApprovedOpenValueIR,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> Option<String> {
    let ApprovedOpenValueIR::Lexical(value) = value else {
        return None;
    };
    let label = value.canonical_lexical_label.trim();
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let display_subject = subject
        .strip_suffix(" 상태")
        .map(str::trim_end)
        .filter(|candidate| !candidate.is_empty())
        .unwrap_or(subject);
    let topic = format!(
        "{display_subject}{}",
        object_particle(display_subject, "은", "는").0
    );
    // These are completed/established state labels, not guessed actions.  A
    // finite predicate removes the mechanically repeated `…된 상태입니다`
    // while preserving the approved status value and its inverse contract.
    if label != "조건부 완료" {
        if let Some((base, predicate_stem)) = [
            (" 완료됨", "완료"),
            (" 완료", "완료"),
            (" 확보", "확보"),
            (" 확인", "확인"),
        ]
        .into_iter()
        .find_map(|(suffix, predicate_stem)| {
            label
                .strip_suffix(suffix)
                .map(str::trim_end)
                .filter(|base| !base.is_empty())
                .map(|base| (base, predicate_stem))
        }) {
            let base_subject = with_particle(base, "이", "가");
            let predicate = match speech_act {
                ApprovedSpeechActIR::Query if formal => format!("{predicate_stem}됐습니까"),
                ApprovedSpeechActIR::Query if casual => format!("{predicate_stem}됐어"),
                ApprovedSpeechActIR::Query => format!("{predicate_stem}됐나요"),
                ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
                _ if formal => format!("{predicate_stem}됐습니다"),
                _ if casual => format!("{predicate_stem}됐어"),
                _ => format!("{predicate_stem}됐어요"),
            };
            return Some(format!("{topic} {base_subject} {predicate}"));
        }
    }
    if let Some(absent) = label
        .strip_suffix(" 없음")
        .map(str::trim_end)
        .filter(|absent| !absent.is_empty())
    {
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "없습니까",
            ApprovedSpeechActIR::Query if casual => "없어",
            ApprovedSpeechActIR::Query => "없나요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "없습니다",
            _ if casual => "없어",
            _ => "없어요",
        };
        let absent_subject = with_particle(absent, "이", "가");
        if let Some(context) = subject
            .strip_suffix(" 결과 상태")
            .map(str::trim_end)
            .filter(|context| !context.is_empty())
        {
            return Some(format!("{context}에서는 {absent_subject} {predicate}"));
        }
        return Some(format!("{topic} {absent_subject} {predicate}"));
    }
    if let Some(boundary) = label
        .strip_suffix(" 일시 중지")
        .map(str::trim_end)
        .filter(|boundary| boundary.ends_with(" 전"))
    {
        let display_subject = subject
            .strip_suffix(" 상태")
            .map(str::trim_end)
            .filter(|candidate| !candidate.is_empty())
            .unwrap_or(subject);
        let display_topic = format!(
            "{display_subject}{}",
            object_particle(display_subject, "은", "는").0
        );
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "일시 중지됩니까",
            ApprovedSpeechActIR::Query if casual => "일시 중지돼",
            ApprovedSpeechActIR::Query => "일시 중지되나요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "일시 중지됩니다",
            _ if casual => "일시 중지돼",
            _ => "일시 중지돼요",
        };
        return Some(format!("{display_topic} {boundary}까지 {predicate}"));
    }
    if let Some(measurement) = label
        .strip_suffix(" 준수")
        .map(str::trim_end)
        .filter(|measurement| !measurement.is_empty())
        .filter(|_| subject.ends_with(" 기준") || subject.ends_with(" 기준 상태"))
    {
        let standard = subject
            .strip_suffix(" 상태")
            .map(str::trim_end)
            .filter(|standard| !standard.is_empty())
            .unwrap_or(subject);
        let standard_object = with_particle(standard, "을", "를");
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "준수합니까",
            ApprovedSpeechActIR::Query if casual => "준수해",
            ApprovedSpeechActIR::Query => "준수하나요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "준수합니다",
            _ if casual => "준수해",
            _ => "준수해요",
        };
        return Some(format!(
            "{measurement}{} {standard_object} {predicate}",
            object_particle(measurement, "은", "는").0
        ));
    }
    if let Some(operation) = label
        .strip_prefix("조건부 ")
        .map(str::trim_start)
        .and_then(|rest| rest.strip_suffix(" 가능"))
        .map(str::trim_end)
        .filter(|operation| !operation.is_empty())
    {
        let display_subject = subject
            .strip_suffix(" 상태")
            .map(str::trim_end)
            .filter(|candidate| !candidate.is_empty())
            .unwrap_or(subject);
        let display_topic = format!(
            "{display_subject}{}",
            object_particle(display_subject, "은", "는").0
        );
        if operation == "운영" {
            let predicate = match speech_act {
                ApprovedSpeechActIR::Query if formal => "조건부로 운영할 수 있습니까",
                ApprovedSpeechActIR::Query if casual => "조건부로 운영할 수 있어",
                ApprovedSpeechActIR::Query => "조건부로 운영할 수 있나요",
                ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
                _ if formal => "조건부로 운영할 수 있습니다",
                _ if casual => "조건부로 운영할 수 있어",
                _ => "조건부로 운영할 수 있어요",
            };
            return Some(format!("{display_topic} {predicate}"));
        }
        let operation_subject = with_particle(&format!("조건부 {operation}"), "이", "가");
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "가능합니까",
            ApprovedSpeechActIR::Query if casual => "가능해",
            ApprovedSpeechActIR::Query => "가능한가요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "가능합니다",
            _ if casual => "가능해",
            _ => "가능해요",
        };
        return Some(format!("{display_topic} {operation_subject} {predicate}"));
    }
    if let Some(condition) = label
        .strip_prefix("조건부 ")
        .map(str::trim_start)
        .and_then(|rest| rest.strip_suffix(" 운영"))
        .map(str::trim_end)
        .filter(|condition| !condition.is_empty())
    {
        let display_subject = subject
            .strip_suffix(" 상태")
            .map(str::trim_end)
            .filter(|candidate| !candidate.is_empty())
            .unwrap_or(subject);
        let display_topic = format!(
            "{display_subject}{}",
            object_particle(display_subject, "은", "는").0
        );
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "됩니까",
            ApprovedSpeechActIR::Query if casual => "돼",
            ApprovedSpeechActIR::Query => "되나요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "됩니다",
            _ if casual => "돼",
            _ => "돼요",
        };
        return Some(format!(
            "{display_topic} 조건부로 {condition} 운영{predicate}"
        ));
    }
    if let Some(location) = label
        .strip_suffix(" 보존")
        .map(str::trim_end)
        .filter(|location| {
            ["위치", "장소", "구역", "지점"]
                .iter()
                .any(|suffix| location.ends_with(suffix))
        })
    {
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "보존됩니까",
            ApprovedSpeechActIR::Query if casual => "보존돼",
            ApprovedSpeechActIR::Query => "보존되나요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "보존됩니다",
            _ if casual => "보존돼",
            _ => "보존돼요",
        };
        return Some(format!("{topic} {location}에 {predicate}"));
    }
    if let Some(preserved) = label
        .strip_suffix(" 보존")
        .map(str::trim_end)
        .filter(|preserved| !preserved.is_empty())
    {
        let container = subject
            .strip_suffix(" 상태")
            .map(str::trim_end)
            .filter(|container| !container.is_empty())
            .unwrap_or(subject);
        let preserved_topic = format!(
            "{container}의 {preserved}{}",
            object_particle(preserved, "은", "는").0
        );
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "보존됩니까",
            ApprovedSpeechActIR::Query if casual => "보존돼",
            ApprovedSpeechActIR::Query => "보존되나요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "보존됩니다",
            _ if casual => "보존돼",
            _ => "보존돼요",
        };
        return Some(format!("{preserved_topic} {predicate}"));
    }
    if let Some(decision) = label
        .strip_suffix(" 확정")
        .map(str::trim_end)
        .filter(|decision| !decision.is_empty())
        .filter(|_| subject.ends_with(" 안건") || subject.ends_with(" 안건 상태"))
    {
        let meeting = subject
            .strip_suffix(" 안건 상태")
            .map(str::trim_end)
            .filter(|meeting| !meeting.is_empty())
            .unwrap_or(subject)
            .to_string();
        let meeting_topic = format!("{meeting}{}", object_particle(&meeting, "은", "는").0);
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "확정하는 것입니까",
            ApprovedSpeechActIR::Query if casual => "확정하는 거야",
            ApprovedSpeechActIR::Query => "확정하는 것인가요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "확정하는 것입니다",
            _ if casual => "확정하는 거야",
            _ => "확정하는 거예요",
        };
        return Some(format!("{meeting_topic} {decision}을 {predicate}"));
    }
    if label == "조건부 완료" {
        let display_subject = subject
            .strip_suffix(" 상태")
            .map(str::trim_end)
            .filter(|candidate| !candidate.is_empty())
            .unwrap_or(subject);
        let display_topic = format!(
            "{display_subject}{}",
            object_particle(display_subject, "은", "는").0
        );
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "조건부로 완료됐습니까",
            ApprovedSpeechActIR::Query if casual => "조건부로 완료됐어",
            ApprovedSpeechActIR::Query => "조건부로 완료됐나요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "조건부로 완료됐습니다",
            _ if casual => "조건부로 완료됐어",
            _ => "조건부로 완료됐어요",
        };
        return Some(format!("{display_topic} {predicate}"));
    }
    if let Some(predicate_stem) = match label {
        "완료됨" | "완료" => Some("완료"),
        "확보" => Some("확보"),
        "확인" => Some("확인"),
        "차단됨" => Some("차단"),
        _ => None,
    } {
        let display_subject = subject
            .strip_suffix(" 상태")
            .map(str::trim_end)
            .filter(|candidate| !candidate.is_empty())
            .unwrap_or(subject);
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => format!("{predicate_stem}됐습니까"),
            ApprovedSpeechActIR::Query if casual => format!("{predicate_stem}됐어"),
            ApprovedSpeechActIR::Query => format!("{predicate_stem}됐나요"),
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => format!("{predicate_stem}됐습니다"),
            _ if casual => format!("{predicate_stem}됐어"),
            _ => format!("{predicate_stem}됐어요"),
        };
        return Some(format!(
            "{} {predicate}",
            with_particle(display_subject, "이", "가")
        ));
    }
    if let Some(scope) = label
        .strip_suffix(" 제한")
        .map(str::trim_end)
        .filter(|scope| scope.ends_with("으로") || scope.ends_with("로"))
    {
        let display_subject = subject
            .strip_suffix(" 상태")
            .map(str::trim_end)
            .filter(|candidate| !candidate.is_empty())
            .unwrap_or(subject);
        let display_topic = format!(
            "{display_subject}{}",
            object_particle(display_subject, "은", "는").0
        );
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "제한됩니까",
            ApprovedSpeechActIR::Query if casual => "제한돼",
            ApprovedSpeechActIR::Query => "제한되나요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "제한됩니다",
            _ if casual => "제한돼",
            _ => "제한돼요",
        };
        return Some(format!("{display_topic} {scope} {predicate}"));
    }
    if label == "필요" {
        let need_subject = with_particle(subject, "이", "가");
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "필요합니까",
            ApprovedSpeechActIR::Query if casual => "필요해",
            ApprovedSpeechActIR::Query => "필요한가요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "필요합니다",
            _ if casual => "필요해",
            _ => "필요해요",
        };
        return Some(format!("{need_subject} {predicate}"));
    }
    if label == "미확인" {
        let predicate = match speech_act {
            ApprovedSpeechActIR::Query if formal => "확인되지 않았습니까",
            ApprovedSpeechActIR::Query if casual => "확인되지 않았어",
            ApprovedSpeechActIR::Query => "확인되지 않았나요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "확인되지 않았습니다",
            _ if casual => "확인되지 않았어",
            _ => "확인되지 않았어요",
        };
        return Some(format!("{topic} {predicate}"));
    }
    let predicate = if label == "통과" {
        match speech_act {
            ApprovedSpeechActIR::Query if formal => "통과했습니까",
            ApprovedSpeechActIR::Query if casual => "통과했어",
            ApprovedSpeechActIR::Query => "통과했나요",
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => "통과했습니다",
            _ if casual => "통과했어",
            _ => "통과했어요",
        }
        .to_string()
    } else if let Some(base) = label
        .strip_suffix(" 필요")
        .map(str::trim_end)
        .filter(|base| !base.is_empty())
    {
        let need_subject = if base.ends_with(" 전") {
            format!("{base}에")
        } else if ["까지", "부터", "이후", "이전", "동안"]
            .iter()
            .any(|suffix| base.ends_with(suffix))
        {
            base.to_string()
        } else {
            with_particle(base, "이", "가")
        };
        match speech_act {
            ApprovedSpeechActIR::Query if formal => format!("{need_subject} 필요합니까"),
            ApprovedSpeechActIR::Query if casual => format!("{need_subject} 필요해"),
            ApprovedSpeechActIR::Query => format!("{need_subject} 필요한가요"),
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
            _ if formal => format!("{need_subject} 필요합니다"),
            _ if casual => format!("{need_subject} 필요해"),
            _ => format!("{need_subject} 필요해요"),
        }
    } else {
        return None;
    };
    Some(format!("{topic} {predicate}"))
}

/// Realize an approved Impact whose lexical value explicitly encodes a need.
/// The causal direction is supplied by the Impact relation; this does not
/// derive a new cause or an unapproved action from the surface label.
fn korean_impact_need_clause(
    subject: &str,
    value: &ApprovedOpenValueIR,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> Option<String> {
    let ApprovedOpenValueIR::Lexical(value) = value else {
        return None;
    };
    let needed = value
        .canonical_lexical_label
        .trim()
        .strip_suffix(" 필요")
        .map(str::trim_end)
        .filter(|needed| !needed.is_empty())?;
    let impacted = subject
        .strip_suffix(" 영향")
        .map(str::trim_end)
        .filter(|candidate| !candidate.is_empty())
        .unwrap_or(subject);
    let predicate = match speech_act {
        ApprovedSpeechActIR::Query if register == LanguageRegisterIR::Formal => "필요합니까",
        ApprovedSpeechActIR::Query
            if matches!(
                register,
                LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
            ) =>
        {
            "필요해"
        }
        ApprovedSpeechActIR::Query => "필요한가요",
        ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
        _ if register == LanguageRegisterIR::Formal => "필요합니다",
        _ if matches!(
            register,
            LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
        ) =>
        {
            "필요해"
        }
        _ => "필요해요",
    };
    Some(format!(
        "{}의 영향으로 {} {predicate}",
        impacted,
        with_particle(needed, "이", "가")
    ))
}

/// Realize an approved nominal Action relation as an execution frame without
/// guessing a domain verb from the value. The action and its content remain
/// separate canonical fields; `진행하다` only realizes the already-approved
/// Action relation itself.
/// A typed action may refer to a separately approved event node.  Unlike a
/// nominal action value, this binding carries an already validated predicate
/// sense and valency frame; the renderer must never infer either from Korean
/// words in the action label.
fn typed_action_event_node_id<'a>(
    claim: &'a ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> Option<&'a str> {
    let ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
        node_id,
        semantic_type: ApprovedSemanticTypeIR::Event,
        ..
    }) = &claim.value
    else {
        return None;
    };
    (claim.relation == ApprovedRelationTypeIR::Action
        && claim.polarity
        && response
            .event_realizations
            .iter()
            .any(|event| event.subject_node_id == *node_id && event.predicate_sense.is_some()))
    .then_some(node_id.as_str())
}

/// Ordered procedure steps are realized with a closed Korean sequence marker
/// only when Canonical IR contains an explicit event dependency.  The marker
/// is derived from node identity and `EarlierThan`, never from an action label
/// or from list position alone.
fn procedure_sequence_marker(
    action: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> Option<&'static str> {
    if action.modality != ApprovedModalityIR::Directive {
        return None;
    }
    let event_id = typed_action_event_node_id(action, response)?;
    let mut has_predecessor = false;
    let mut has_successor = false;
    for dependency in response
        .claims
        .iter()
        .filter(|claim| claim.relation == ApprovedRelationTypeIR::EarlierThan && claim.polarity)
    {
        if dependency.subject.node_id == event_id {
            has_successor = true;
        }
        if matches!(
            &dependency.value,
            ApprovedOpenValueIR::Lexical(reference) if reference.node_id == event_id
        ) {
            has_predecessor = true;
        }
    }
    match (language, has_predecessor, has_successor) {
        (LanguageCodeIR::Korean, false, true) => Some("먼저"),
        (LanguageCodeIR::Korean, true, true) => Some("이어서"),
        (LanguageCodeIR::Korean, true, false) => Some("그다음"),
        (_, false, true) => Some("First,"),
        (_, true, true) => Some("Next,"),
        (_, true, false) => Some("Then,"),
        (_, false, false) => None,
    }
}

fn is_directive_procedure_dependency(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> bool {
    let ApprovedOpenValueIR::Lexical(successor) = &claim.value else {
        return false;
    };
    claim.relation == ApprovedRelationTypeIR::EarlierThan
        && claim.polarity
        && response.claims.iter().any(|action| {
            action.relation == ApprovedRelationTypeIR::Action
                && action.modality == ApprovedModalityIR::Directive
                && typed_action_event_node_id(action, response)
                    .is_some_and(|event_id| event_id == claim.subject.node_id)
        })
        && response.claims.iter().any(|action| {
            action.relation == ApprovedRelationTypeIR::Action
                && action.modality == ApprovedModalityIR::Directive
                && typed_action_event_node_id(action, response)
                    .is_some_and(|event_id| event_id == successor.node_id)
        })
}

/// A complete typed event is the Korean realization of the source-authoritative
/// Action→event relation.  The relation remains in Canonical IR and is restored
/// by `parse_typed_action_event_sentence`; Korean does not repeat a nominal
/// action label when the predicate and all of its valency are already stated.
fn korean_typed_action_event_clause(event_surface: &str) -> Option<String> {
    let event_clause = event_surface.trim().trim_end_matches('.').trim();
    (!event_clause.is_empty()).then(|| event_clause.to_string())
}

fn korean_action_relation_clause(
    subject: &str,
    value: &ApprovedOpenValueIR,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> Option<String> {
    let ApprovedOpenValueIR::Lexical(_) = value else {
        return None;
    };
    if matches!(
        speech_act,
        ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise
    ) {
        return None;
    }
    let action_subject = if subject.ends_with("조치") {
        subject.to_string()
    } else {
        format!("{subject} 조치")
    };
    let topic = object_particle(&action_subject, "은", "는").0;
    let displayed_value = display_value(value, LanguageCodeIR::Korean, register);
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let predicate = match speech_act {
        ApprovedSpeechActIR::Query if formal => "진행됩니까",
        ApprovedSpeechActIR::Query if casual => "진행돼",
        ApprovedSpeechActIR::Query => "진행되나요",
        _ if formal => "진행됩니다",
        _ if casual => "진행돼",
        _ => "진행돼요",
    };
    // A locative/directional nominal predicate such as `자료실로 전환` already
    // carries its own `로/으로` phrase.  Attaching a second directional particle
    // produces `전환으로 진행`, which is grammatical only as a clipped note and
    // sounds repetitive in a complete sentence.  This small closed morphology
    // class changes only the realization frame of the approved Action relation;
    // it neither infers a domain predicate nor changes the action value.
    let execution_complement = ["전환", "운영", "시공"]
        .iter()
        .any(|ending| displayed_value.ends_with(ending))
        .then(|| format!("{displayed_value}하는 방식으로"));
    if let Some(complement) = execution_complement {
        return Some(format!("{action_subject}{topic} {complement} {predicate}"));
    }
    let complement = directional_particle(particle_basis(value, &displayed_value)).0;
    Some(format!(
        "{action_subject}{topic} {displayed_value}{complement} {predicate}"
    ))
}

/// A concrete canonical subject with the Location relation is realized as an
/// ordinary locative clause. Event subjects remain on the generic event-place
/// path unless they explicitly carry a location head (`X 장소`), because an
/// event and a physical entity take different Korean location constructions.
/// The relation and semantic type are supplied by canonical IR; this never
/// infers installation, ownership, or a new event for `X`.
fn korean_embedded_location_clause(
    subject: &str,
    subject_type: ApprovedSemanticTypeIR,
    value: &ApprovedOpenValueIR,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> Option<String> {
    let ApprovedOpenValueIR::Lexical(_) = value else {
        return None;
    };
    // `보관 위치`, `설치 위치` and similar compound locative heads are a
    // single nominal unit.  Removing only `위치` leaves an unlicensed Korean
    // predicate noun (`검체 상자 보관은 …에 있습니다`).  Keep the complete
    // canonical subject; this changes neither the location relation nor any
    // event interpretation.
    let compound_location_head = [
        " 보관 위치",
        " 설치 위치",
        " 배치 위치",
        " 적재 위치",
        " 보존 위치",
        " 대기 위치",
    ]
    .iter()
    .any(|suffix| subject.ends_with(suffix));
    let embedded_location_subject = compound_location_head
        .then_some(subject)
        .or_else(|| subject.strip_suffix(" 장소").map(str::trim_end))
        .or_else(|| subject.strip_suffix(" 위치").map(str::trim_end))
        .filter(|located| !located.is_empty());
    let located = embedded_location_subject.or(match subject_type {
        ApprovedSemanticTypeIR::Concept | ApprovedSemanticTypeIR::Location => Some(subject),
        _ => None,
    })?;
    if matches!(
        speech_act,
        ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise
    ) {
        return None;
    }
    let displayed_value = display_value(value, LanguageCodeIR::Korean, register);
    let topic = object_particle(located, "은", "는").0;
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let predicate = match speech_act {
        ApprovedSpeechActIR::Query if formal => "있습니까",
        ApprovedSpeechActIR::Query if casual => "있어",
        ApprovedSpeechActIR::Query => "있나요",
        _ if formal => "있습니다",
        _ if casual => "있어",
        _ => "있어요",
    };
    Some(format!("{located}{topic} {displayed_value}에 {predicate}"))
}

/// Realize lexical progress states without repeating the same process noun.
/// For example, a subject `화면 설계 검토` with value `검토 중` is realized as
/// `화면 설계는 검토 중입니다`, while unrelated pairs such as
/// `결제 서비스` + `점검 중` keep the full subject.  The normalization is
/// structural and shared by realization and semantic inverse.
fn korean_progress_status_clause(
    subject: &str,
    value: &ApprovedOpenValueIR,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> Option<String> {
    let ApprovedOpenValueIR::Lexical(value) = value else {
        return None;
    };
    let value = value.canonical_lexical_label.trim();
    let process = value.strip_suffix(" 중")?.trim_end();
    let display_subject = subject
        .strip_suffix(process)
        .map(str::trim_end)
        .filter(|candidate| !candidate.is_empty())
        .unwrap_or(subject);
    let topic = object_particle(display_subject, "은", "는").0;
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let ending = match speech_act {
        ApprovedSpeechActIR::Query if formal => "입니까",
        ApprovedSpeechActIR::Query if casual => "이야",
        ApprovedSpeechActIR::Query => "인가요",
        ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise => return None,
        _ if formal => "입니다",
        _ if casual => "이야",
        _ => "이에요",
    };
    Some(format!("{display_subject}{topic} {value}{ending}"))
}

fn korean_count_speech_act_clause(
    subject: &str,
    amount: i64,
    classifier: &str,
    speech_act: ApprovedSpeechActIR,
    operation: ApprovedOperationIR,
    register: LanguageRegisterIR,
) -> String {
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let topic = object_particle(subject, "은", "는").0;
    let quantity = format!("{amount}{classifier}");
    if operation == ApprovedOperationIR::Revise {
        return format!(
            "{subject}{topic} {} {}",
            with_directional_particle(&quantity),
            if formal {
                "변경됐습니다"
            } else if casual {
                "바뀌었어"
            } else {
                "바뀌었어요"
            }
        );
    }
    match speech_act {
        ApprovedSpeechActIR::Query => format!(
            "{subject}{topic} {quantity}{}",
            if formal {
                "입니까"
            } else if casual {
                copula_casual(&quantity)
            } else {
                "인가요"
            }
        ),
        ApprovedSpeechActIR::Request => format!(
            "{subject} 수량을 {} {}",
            with_directional_particle(&quantity),
            if formal {
                "맞춰 주십시오"
            } else if casual {
                "맞춰 줘"
            } else {
                "맞춰 주세요"
            }
        ),
        ApprovedSpeechActIR::Promise => format!(
            "{subject} 수량을 {} {}",
            with_directional_particle(&quantity),
            if formal {
                "맞추겠습니다"
            } else if casual {
                "맞출게"
            } else {
                "맞출게요"
            }
        ),
        _ => format!(
            "{subject}{topic} {quantity}{}",
            if formal {
                "입니다"
            } else if casual {
                copula_casual(&quantity)
            } else {
                copula_yo(&quantity)
            }
        ),
    }
}

fn korean_reassurance_marker(register: LanguageRegisterIR) -> &'static str {
    match register {
        LanguageRegisterIR::Formal => "안심하셔도 됩니다.",
        LanguageRegisterIR::Neutral => "안심하셔도 돼요.",
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet => "안심해도 돼.",
    }
}

fn korean_negative_typed_state_speech_act_clause(
    subject: &str,
    relation: ApprovedRelationTypeIR,
    lexical_value: &str,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> Option<String> {
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let topic = format!("{subject}{}", object_particle(subject, "은", "는").0);
    let object = format!("{subject}{}", object_particle(subject, "을", "를").0);
    match (relation, lexical_value, speech_act) {
        (ApprovedRelationTypeIR::Status, "준비", ApprovedSpeechActIR::Query) => Some(format!(
            "{topic} {}",
            if formal {
                "준비되지 않았습니까"
            } else if casual {
                "준비되지 않았어"
            } else {
                "준비되지 않았나요"
            }
        )),
        (ApprovedRelationTypeIR::Status, "준비", ApprovedSpeechActIR::Request) => Some(format!(
            "{object} {}",
            if formal {
                "준비하지 말아 주십시오"
            } else if casual {
                "준비하지 마"
            } else {
                "준비하지 말아 주세요"
            }
        )),
        (ApprovedRelationTypeIR::Status, "준비", ApprovedSpeechActIR::Promise) => Some(format!(
            "{object} {}",
            if formal {
                "준비하지 않겠습니다"
            } else if casual {
                "준비하지 않을게"
            } else {
                "준비하지 않을게요"
            }
        )),
        (ApprovedRelationTypeIR::Status, "준비", _) => Some(format!(
            "{topic} {}",
            if formal {
                "준비되지 않았습니다"
            } else if casual {
                "준비되지 않았어"
            } else {
                "준비되지 않았어요"
            }
        )),
        (ApprovedRelationTypeIR::Registration, "완료", ApprovedSpeechActIR::Query) => {
            Some(format!(
                "{subject} 등록이 {}",
                if formal {
                    "완료되지 않았습니까"
                } else if casual {
                    "완료되지 않았어"
                } else {
                    "완료되지 않았나요"
                }
            ))
        }
        (ApprovedRelationTypeIR::Registration, "완료", ApprovedSpeechActIR::Request) => {
            Some(format!(
                "{subject} 등록을 {}",
                if formal {
                    "완료하지 말아 주십시오"
                } else if casual {
                    "완료하지 마"
                } else {
                    "완료하지 말아 주세요"
                }
            ))
        }
        (ApprovedRelationTypeIR::Registration, "완료", ApprovedSpeechActIR::Promise) => {
            Some(format!(
                "{subject} 등록을 {}",
                if formal {
                    "완료하지 않겠습니다"
                } else if casual {
                    "완료하지 않을게"
                } else {
                    "완료하지 않을게요"
                }
            ))
        }
        (ApprovedRelationTypeIR::Registration, "완료", _) => Some(format!(
            "{subject} 등록이 {}",
            if formal {
                "완료되지 않았습니다"
            } else if casual {
                "완료되지 않았어"
            } else {
                "완료되지 않았어요"
            }
        )),
        _ => None,
    }
}

fn korean_negative_value_speech_act_clause(
    subject: &str,
    relation: ApprovedRelationTypeIR,
    value: &ApprovedOpenValueIR,
    displayed_value: &str,
    speech_act: ApprovedSpeechActIR,
    register: LanguageRegisterIR,
) -> String {
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let relation_surface = relation_label(relation, LanguageCodeIR::Korean);
    let topic = object_particle(relation_surface, "은", "는").0;
    let property = format!("{subject} {relation_surface}{topic}");
    let object = object_particle(relation_surface, "을", "를").0;
    let property_object = format!("{subject} {relation_surface}{object}");
    let nominative = object_particle(particle_basis(value, displayed_value), "이", "가")
        .prepend(displayed_value);
    let directional =
        directional_particle(particle_basis(value, displayed_value)).prepend(displayed_value);

    match speech_act {
        ApprovedSpeechActIR::Query => format!(
            "{property}{nominative} {}",
            if formal {
                "아닙니까"
            } else if casual {
                "아니야"
            } else {
                "아닌가요"
            }
        ),
        ApprovedSpeechActIR::Request if relation == ApprovedRelationTypeIR::Status => format!(
            "{subject} 상태를{directional} {}",
            if formal {
                "설정하지 말아 주십시오"
            } else if casual {
                "설정하지 마"
            } else {
                "설정하지 말아 주세요"
            }
        ),
        ApprovedSpeechActIR::Request => format!(
            "{property_object}{directional} {}",
            if formal {
                "하지 말아 주십시오"
            } else if casual {
                "하지 마"
            } else {
                "하지 말아 주세요"
            }
        ),
        ApprovedSpeechActIR::Promise if relation == ApprovedRelationTypeIR::Status => format!(
            "{subject} 상태를{directional} {}",
            if formal {
                "설정하지 않겠습니다"
            } else if casual {
                "설정하지 않을게"
            } else {
                "설정하지 않을게요"
            }
        ),
        ApprovedSpeechActIR::Promise => format!(
            "{property_object}{directional} {}",
            if formal {
                "하지 않겠습니다"
            } else if casual {
                "하지 않을게"
            } else {
                "하지 않을게요"
            }
        ),
        _ => format!(
            "{property}{nominative} {}",
            if formal {
                "아닙니다"
            } else if casual {
                "아니야"
            } else {
                "아니에요"
            }
        ),
    }
}

fn english_boolean_predicates() -> Vec<(ApprovedRelationTypeIR, bool, &'static str)> {
    vec![
        (ApprovedRelationTypeIR::Cancelled, true, "is cancelled"),
        (ApprovedRelationTypeIR::Cancelled, false, "is not cancelled"),
        (ApprovedRelationTypeIR::RoomAvailable, true, "is available"),
        (
            ApprovedRelationTypeIR::RoomAvailable,
            false,
            "is not available",
        ),
        (ApprovedRelationTypeIR::Registration, true, "is registered"),
        (
            ApprovedRelationTypeIR::Registration,
            false,
            "is not registered",
        ),
        (ApprovedRelationTypeIR::Entry, true, "allows entry"),
        (ApprovedRelationTypeIR::Entry, false, "does not allow entry"),
        (ApprovedRelationTypeIR::Confirmed, true, "is confirmed"),
        (ApprovedRelationTypeIR::Confirmed, false, "is not confirmed"),
        (ApprovedRelationTypeIR::Approved, true, "is approved"),
        (ApprovedRelationTypeIR::Approved, false, "is not approved"),
    ]
}

fn korean_truth_predicate(predicate: &str) -> Option<bool> {
    match predicate {
        "맞습니다" | "맞아요" | "맞아" => Some(true),
        "아닙니다" | "아니에요" | "아니야" => Some(false),
        _ => None,
    }
}

fn parse_korean_particle_value(
    surface: &str,
    consonant: &'static str,
    vowel: &'static str,
    response: &ApprovedCompositionalResponseIR,
) -> Vec<ApprovedOpenValueIR> {
    let mut values = Vec::new();
    for particle in [consonant, vowel] {
        if let Some(value_surface) = surface.strip_suffix(particle) {
            if !value_surface.is_empty() {
                for value in parse_open_values(value_surface, response, LanguageCodeIR::Korean) {
                    let expected_particle = if consonant == "으로" && vowel == "로" {
                        directional_particle(particle_basis(&value, value_surface)).0
                    } else {
                        object_particle(particle_basis(&value, value_surface), consonant, vowel).0
                    };
                    if expected_particle != particle {
                        continue;
                    }
                    if !values.contains(&value) {
                        values.push(value);
                    }
                }
            }
        }
    }
    values
}

fn parse_open_values(
    surface: &str,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> Vec<ApprovedOpenValueIR> {
    let surface = surface.trim();
    if surface.is_empty() {
        return Vec::new();
    }
    if let Some(text) = decode_text_value(surface) {
        return vec![ApprovedOpenValueIR::Text(text)];
    }
    let mut values = Vec::new();
    if let Some((hour, minute)) = parse_clock(surface, language) {
        values.push(ApprovedOpenValueIR::Clock { hour, minute });
    }
    if let Some((year, month, day)) = parse_date(surface, language) {
        values.push(ApprovedOpenValueIR::Date { year, month, day });
    }
    if let Ok(value) = surface.parse::<i64>() {
        values.push(ApprovedOpenValueIR::Integer(value));
    }
    for claim in &response.claims {
        if matches!(claim.value, ApprovedOpenValueIR::Integer(_)) {
            let classifier = match claim.relation {
                ApprovedRelationTypeIR::Capacity => Some("명"),
                ApprovedRelationTypeIR::Count => Some("개"),
                _ => None,
            };
            if let Some(classifier) = classifier {
                if let Some(amount_surface) = surface.strip_suffix(classifier) {
                    if let Ok(amount) = amount_surface.parse::<i64>() {
                        let integer = ApprovedOpenValueIR::Integer(amount);
                        if !values.contains(&integer) {
                            values.push(integer);
                        }
                    }
                }
            }
        }
        if let ApprovedOpenValueIR::Lexical(node) = &claim.value {
            let lexical_surfaces = match language {
                LanguageCodeIR::Korean => vec![
                    display_korean_lexical_value(&node.canonical_lexical_label),
                    display_korean_status_value(&node.canonical_lexical_label),
                ],
                _ => vec![node.canonical_lexical_label.clone()],
            };
            if (node.canonical_lexical_label == surface
                || lexical_surfaces
                    .iter()
                    .any(|candidate| candidate == surface))
                && !values.contains(&ApprovedOpenValueIR::Lexical(node.clone()))
            {
                values.push(ApprovedOpenValueIR::Lexical(node.clone()));
            }
        }
        if let ApprovedOpenValueIR::Quantity { unit, .. } = &claim.value {
            let amount_surface = match language {
                LanguageCodeIR::Korean => surface.strip_suffix(&unit.canonical_lexical_label),
                _ => surface
                    .strip_suffix(&unit.canonical_lexical_label)
                    .and_then(|prefix| prefix.strip_suffix(' ')),
            };
            if let Some(amount_surface) = amount_surface {
                if let Ok(amount) = amount_surface.parse::<i64>() {
                    let quantity = ApprovedOpenValueIR::Quantity {
                        amount,
                        unit: unit.clone(),
                    };
                    if !values.contains(&quantity) {
                        values.push(quantity);
                    }
                }
            }
        }
    }
    values
}

fn parse_clock(surface: &str, language: LanguageCodeIR) -> Option<(u8, u8)> {
    match language {
        LanguageCodeIR::Korean => {
            let (period, remainder) = surface.split_once(' ')?;
            if !matches!(period, "오전" | "오후") {
                return None;
            }
            let (hour_surface, minute) = if let Some((hour, minute)) = remainder.split_once(' ') {
                (
                    hour.strip_suffix('시')?,
                    minute.strip_suffix('분')?.parse().ok()?,
                )
            } else {
                (remainder.strip_suffix('시')?, 0)
            };
            let display_hour = hour_surface.parse::<u8>().ok()?;
            if !(1..=12).contains(&display_hour) || minute >= 60 {
                return None;
            }
            let hour = match (period, display_hour) {
                ("오전", 12) => 0,
                ("오전", hour) => hour,
                ("오후", 12) => 12,
                ("오후", hour) => hour + 12,
                _ => return None,
            };
            Some((hour, minute))
        }
        _ => {
            let (hour, minute) = surface.split_once(':')?;
            let hour = hour.parse::<u8>().ok()?;
            let minute = minute.parse::<u8>().ok()?;
            (hour < 24 && minute < 60).then_some((hour, minute))
        }
    }
}

fn parse_date(surface: &str, language: LanguageCodeIR) -> Option<(i32, u8, u8)> {
    match language {
        LanguageCodeIR::Korean => {
            let (year, remainder) = surface.split_once("년 ")?;
            let (month, day) = remainder.split_once("월 ")?;
            let year = year.parse::<i32>().ok()?;
            let month = month.parse::<u8>().ok()?;
            let day = day.strip_suffix('일')?.parse::<u8>().ok()?;
            ((1..=9999).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day))
                .then_some((year, month, day))
        }
        _ => {
            let mut parts = surface.split('-');
            let year = parts.next()?.parse::<i32>().ok()?;
            let month = parts.next()?.parse::<u8>().ok()?;
            let day = parts.next()?.parse::<u8>().ok()?;
            (parts.next().is_none()
                && (1..=9999).contains(&year)
                && (1..=12).contains(&month)
                && (1..=31).contains(&day))
            .then_some((year, month, day))
        }
    }
}

fn push_parsed(
    parsed: &mut Vec<ParsedSurfaceClaim>,
    subject: ApprovedLexicalNodeIR,
    relation: ApprovedRelationTypeIR,
    value: ApprovedOpenValueIR,
    polarity: bool,
) {
    let candidate = ParsedSurfaceClaim {
        subject,
        relation,
        value,
        polarity,
    };
    if !parsed.contains(&candidate) {
        parsed.push(candidate);
    }
}

fn effective_claim_value(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> (ApprovedOpenValueIR, bool) {
    match claim.value {
        ApprovedOpenValueIR::Boolean(value) => {
            let asserted = if !claim.polarity || response.operation == ApprovedOperationIR::Negate {
                !value
            } else {
                value
            };
            (ApprovedOpenValueIR::Boolean(asserted), true)
        }
        _ => (
            claim.value.clone(),
            claim.polarity && response.operation != ApprovedOperationIR::Negate,
        ),
    }
}

fn parsed_matches_claim(
    parsed: &ParsedSurfaceClaim,
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> bool {
    let (value, polarity) = effective_claim_value(claim, response);
    parsed.subject == claim.subject
        && parsed.relation == claim.relation
        && parsed.value == value
        && parsed.polarity == polarity
}

fn interpreted_matches_claim(
    interpreted: &DocumentSemanticClaimIR,
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> bool {
    let (value, polarity) = effective_claim_value(claim, response);
    interpreted.subject == claim.subject
        && interpreted.relation == claim.relation
        && interpreted.value == value
        && interpreted.polarity == polarity
        && interpreted.modality == claim.modality
}

#[derive(Debug)]
struct PlannedDocumentSection {
    role: DocumentResponseRoleIR,
    claim_ids: Vec<String>,
}

fn plan_rhetorical_units(
    response: &ApprovedCompositionalResponseIR,
) -> Vec<DocumentRhetoricalUnitIR> {
    let last_index = response.claims.len().saturating_sub(1);
    // An event frame is indivisible for both realization and rhetoric.  If
    // its final argument carries a terminal move while earlier arguments keep
    // a supporting move, the event planner sees a truncated valency frame and
    // falls back to nominal Action output.  Preserve the terminal discourse
    // role across the complete, already typed event bundle; no event or role
    // is inferred from a surface string here.
    let terminal_event_node_id = response
        .claims
        .last()
        .filter(|claim| is_typed_event_argument(claim, response))
        .map(|claim| claim.subject.node_id.as_str());
    response
        .claims
        .iter()
        .enumerate()
        .map(|(index, claim)| {
            let terminal_event_argument = terminal_event_node_id.is_some_and(|node_id| {
                claim.subject.node_id == node_id && is_typed_event_argument(claim, response)
            });
            let rhetorical_move = if terminal_event_argument
                && matches!(
                    response.discourse_relation,
                    ApprovedDiscourseRelationIR::Cause
                        | ApprovedDiscourseRelationIR::Condition
                        | ApprovedDiscourseRelationIR::Explanation
                )
            {
                DocumentRhetoricalMoveIR::Conclusion
            } else {
                match response.discourse_relation {
                ApprovedDiscourseRelationIR::Correction => DocumentRhetoricalMoveIR::Correction,
                ApprovedDiscourseRelationIR::Cause if response.claims.len() > 1 => {
                    if index == last_index
                        && matches!(
                            claim.relation,
                            ApprovedRelationTypeIR::Impact
                                | ApprovedRelationTypeIR::Action
                                | ApprovedRelationTypeIR::Status
                                | ApprovedRelationTypeIR::Cancelled
                                | ApprovedRelationTypeIR::Confirmed
                                | ApprovedRelationTypeIR::Approved
                                | ApprovedRelationTypeIR::ResultState
                        )
                    {
                        DocumentRhetoricalMoveIR::Conclusion
                    } else if index < last_index {
                        DocumentRhetoricalMoveIR::Cause
                    } else {
                        DocumentRhetoricalMoveIR::Assertion
                    }
                }
                ApprovedDiscourseRelationIR::Condition if response.claims.len() > 1 => {
                    if index == last_index {
                        DocumentRhetoricalMoveIR::Conclusion
                    } else {
                        DocumentRhetoricalMoveIR::Condition
                    }
                }
                ApprovedDiscourseRelationIR::Explanation if response.claims.len() > 1 => {
                    if index == last_index
                        && !matches!(
                            claim.relation,
                            ApprovedRelationTypeIR::Owner | ApprovedRelationTypeIR::Deadline
                        )
                    {
                        DocumentRhetoricalMoveIR::Conclusion
                    } else {
                        DocumentRhetoricalMoveIR::Support
                    }
                }
                ApprovedDiscourseRelationIR::Simultaneous if index > 0 => {
                    DocumentRhetoricalMoveIR::Concurrent
                }
                _ => DocumentRhetoricalMoveIR::Assertion,
                }
            };
            DocumentRhetoricalUnitIR {
                unit_index: index,
                proposition_id: claim.proposition_id.clone(),
                rhetorical_move,
                predecessor_indices: (index > 0).then(|| index - 1).into_iter().collect(),
                semantic_authority: false,
            }
        })
        .collect()
}

fn plan_event_predicates(
    response: &ApprovedCompositionalResponseIR,
    sections: &[DocumentResponseSectionIR],
    rhetorical_units: &[DocumentRhetoricalUnitIR],
) -> Vec<DocumentEventPredicateIR> {
    let mut predicates = Vec::new();
    for section in sections {
        let claims = section
            .claim_ids
            .iter()
            .filter_map(|id| {
                response
                    .claims
                    .iter()
                    .find(|claim| claim.proposition_id == *id)
            })
            .collect::<Vec<_>>();
        let mut start = 0usize;
        while start < claims.len() {
            let first = claims[start];
            if !is_typed_event_argument(first, response) {
                start += 1;
                continue;
            }
            // A typed Action frame is one source-authoritative proposition:
            // its time, manner, and other valency roles must remain attached
            // even when their generic rhetorical roles differ.  Otherwise a
            // directive can render its action and then strand a role in a
            // separate sentence, which makes exact inverse recovery depend
            // on paragraph-level parsing rather than the frame contract.
            let action_bound_event = response.claims.iter().any(|action| {
                typed_action_event_node_id(action, response)
                    .is_some_and(|event_id| event_id == first.subject.node_id)
            });
            let first_move = rhetorical_units
                .iter()
                .find(|unit| unit.proposition_id == first.proposition_id)
                .map(|unit| unit.rhetorical_move);
            let mut end = start + 1;
            while end < claims.len() && end - start < MAX_EVENT_ARGUMENTS {
                let candidate = claims[end];
                let candidate_move = rhetorical_units
                    .iter()
                    .find(|unit| unit.proposition_id == candidate.proposition_id)
                    .map(|unit| unit.rhetorical_move);
                if event_argument_order(candidate.relation).is_none() {
                    break;
                }
                if candidate.subject != first.subject
                    || (!action_bound_event && candidate_move != first_move)
                    || !is_typed_event_argument(candidate, response)
                {
                    break;
                }
                end += 1;
            }
            let candidate_claims = &claims[start..end];
            let semantic_siblings_complete = approved_event_predicate(
                response,
                &first.subject.node_id,
            )
            .is_none_or(|predicate| {
                !is_semantic_event_kind(predicate.kind)
                    || response
                        .claims
                        .iter()
                        .filter(|candidate| {
                            let candidate_move = rhetorical_units
                                .iter()
                                .find(|unit| unit.proposition_id == candidate.proposition_id)
                                .map(|unit| unit.rhetorical_move);
                            candidate.subject == first.subject
                                && (action_bound_event || candidate_move == first_move)
                                // A procedure dependency belongs to the event graph, but it
                                // is not an argument of either event.  Other same-subject
                                // claims remain visible here so an unmodeled role fails closed
                                // instead of being silently omitted from a typed predicate.
                                && candidate.relation != ApprovedRelationTypeIR::EarlierThan
                        })
                        .count()
                        == candidate_claims.len()
            });
            if semantic_siblings_complete
                && event_argument_set_complete(first, candidate_claims, response)
            {
                let predicate_index = predicates.len();
                let approved = approved_event_predicate(response, &first.subject.node_id)
                    .expect("typed event argument requires approved realization metadata");
                predicates.push(DocumentEventPredicateIR {
                    predicate_index,
                    subject_node_id: first.subject.node_id.clone(),
                    kind: approved.kind,
                    phase: approved.phase,
                    predicate_sense: approved.predicate_sense,
                    perspective: approved.perspective,
                    voice: approved.voice,
                    information_structure: approved.information_structure,
                    arguments: claims[start..end]
                        .iter()
                        .map(|claim| DocumentEventArgumentIR {
                            proposition_id: claim.proposition_id.clone(),
                            role: event_argument_role(claim.relation).expect("typed argument"),
                        })
                        .collect(),
                    predecessor_predicate_indices: (predicate_index > 0)
                        .then(|| predicate_index - 1)
                        .into_iter()
                        .collect(),
                    semantic_authority: false,
                });
                start = end;
            } else {
                start += 1;
            }
        }
    }
    // Sections are presentation metadata.  Recover complete typed Action
    // frames from the canonical graph when their roles crossed a section
    // boundary, so display grouping cannot degrade a source-bound action into
    // an opaque fallback.
    for action in response
        .claims
        .iter()
        .filter(|claim| claim.relation == ApprovedRelationTypeIR::Action && claim.polarity)
    {
        let Some(event_node_id) = typed_action_event_node_id(action, response) else {
            continue;
        };
        if predicates
            .iter()
            .any(|predicate| predicate.subject_node_id == event_node_id)
        {
            continue;
        }
        let mut arguments = response
            .claims
            .iter()
            .filter(|claim| claim.subject.node_id == event_node_id)
            .collect::<Vec<_>>();
        arguments.sort_by_key(|claim| event_argument_order(claim.relation));
        let Some(first) = arguments.first().copied() else {
            continue;
        };
        if !arguments
            .iter()
            .all(|claim| is_typed_event_argument(claim, response))
            || !event_argument_set_complete(first, &arguments, response)
        {
            continue;
        }
        let approved = approved_event_predicate(response, &event_node_id)
            .expect("typed action event requires approved realization metadata");
        let predicate_index = predicates.len();
        predicates.push(DocumentEventPredicateIR {
            predicate_index,
            subject_node_id: event_node_id.to_owned(),
            kind: approved.kind,
            phase: approved.phase,
            predicate_sense: approved.predicate_sense,
            perspective: approved.perspective,
            voice: approved.voice,
            information_structure: approved.information_structure,
            arguments: arguments
                .iter()
                .map(|claim| DocumentEventArgumentIR {
                    proposition_id: claim.proposition_id.clone(),
                    role: event_argument_role(claim.relation)
                        .expect("typed action event arguments are role-bound"),
                })
                .collect(),
            predecessor_predicate_indices: (predicate_index > 0)
                .then(|| predicate_index - 1)
                .into_iter()
                .collect(),
            semantic_authority: false,
        });
    }
    predicates
}

fn is_typed_event_argument(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> bool {
    let directive_action_for_event = response.claims.iter().any(|action| {
        typed_action_event_node_id(action, response)
            .is_some_and(|node_id| node_id == claim.subject.node_id)
            && action.modality == ApprovedModalityIR::Directive
    });
    let commissive_action_for_event = response.claims.iter().any(|action| {
        typed_action_event_node_id(action, response)
            .is_some_and(|node_id| node_id == claim.subject.node_id)
            && action.modality == ApprovedModalityIR::Commissive
    });
    if claim.subject.semantic_type != ApprovedSemanticTypeIR::Event
        || approved_event_predicate(response, &claim.subject.node_id).is_none()
        || !claim.polarity
        || (!(matches!(
            response.operation,
            ApprovedOperationIR::Assert
                | ApprovedOperationIR::Explain
                | ApprovedOperationIR::Recall
                | ApprovedOperationIR::Confirm
        ) || (response.operation == ApprovedOperationIR::Request && directive_action_for_event)
            || (response.operation == ApprovedOperationIR::Promise && commissive_action_for_event)))
        || matches!(
            response.discourse_relation,
            ApprovedDiscourseRelationIR::Correction
                | ApprovedDiscourseRelationIR::Negation
                | ApprovedDiscourseRelationIR::Comparison
                | ApprovedDiscourseRelationIR::Condition
        )
    {
        return false;
    }
    let Some(approved) = approved_event_predicate(response, &claim.subject.node_id) else {
        return false;
    };
    let scheduled_argument = matches!(
        (&claim.relation, &claim.value),
        (
            ApprovedRelationTypeIR::Date,
            ApprovedOpenValueIR::Date { .. }
        ) | (
            ApprovedRelationTypeIR::Time,
            ApprovedOpenValueIR::Clock { .. } | ApprovedOpenValueIR::Text(_)
        ) | (
            ApprovedRelationTypeIR::Location,
            ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                semantic_type: ApprovedSemanticTypeIR::Location,
                ..
            })
        )
    );
    if !is_semantic_event_kind(approved.kind) {
        return scheduled_argument;
    }
    let Some(predicate_sense) = approved.predicate_sense else {
        return false;
    };
    // A physical destination is a Location, but a transfer can also be sent
    // to an explicitly typed person or organization.  Treating every
    // Destination as a place silently breaks ordinary notification and
    // delivery frames after their source has already supplied a valid sense
    // and recipient role.  This is a role-type distinction, not a lexical
    // exception: Motion keeps its location-only destination contract.
    let recipient_destination = matches!(
        (&claim.relation, &claim.value),
        (
            ApprovedRelationTypeIR::Destination,
            ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                semantic_type: ApprovedSemanticTypeIR::Person
                    | ApprovedSemanticTypeIR::Organization,
                ..
            })
        )
    ) && matches!(
        predicate_sense,
        ApprovedEventPredicateSenseIR::Send
            | ApprovedEventPredicateSenseIR::Deliver
            | ApprovedEventPredicateSenseIR::Give
    );
    (scheduled_argument
        || recipient_destination
        || matches!(
            (&claim.relation, &claim.value),
            (
                ApprovedRelationTypeIR::Agent,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    semantic_type: ApprovedSemanticTypeIR::Person
                        | ApprovedSemanticTypeIR::Organization,
                    ..
                })
            ) | (
                ApprovedRelationTypeIR::Patient | ApprovedRelationTypeIR::Theme,
                ApprovedOpenValueIR::Lexical(_)
            ) | (
                ApprovedRelationTypeIR::Source | ApprovedRelationTypeIR::Destination,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    ..
                })
            ) | (
                ApprovedRelationTypeIR::Target,
                ApprovedOpenValueIR::Lexical(_)
            ) | (
                ApprovedRelationTypeIR::Instrument,
                ApprovedOpenValueIR::Lexical(_)
            ) | (
                ApprovedRelationTypeIR::Manner,
                ApprovedOpenValueIR::Text(_) | ApprovedOpenValueIR::Lexical(_)
            ) | (
                ApprovedRelationTypeIR::InitialState | ApprovedRelationTypeIR::ResultState,
                _
            )
        ))
        && semantic_event_class(approved.kind)
            .and_then(|class| approved_event_predicate_spec(class, predicate_sense))
            .is_some_and(|spec| spec.allows_role(claim.relation))
}

fn event_argument_set_complete(
    first: &ApprovedCompositionalClaimIR,
    claims: &[&ApprovedCompositionalClaimIR],
    response: &ApprovedCompositionalResponseIR,
) -> bool {
    let Some(approved) = approved_event_predicate(response, &first.subject.node_id) else {
        return false;
    };
    if !is_semantic_event_kind(approved.kind) {
        return claims.len() >= 2;
    }
    let Some(information) = normalize_event_information(
        approved.perspective,
        approved.voice,
        approved.information_structure.as_ref(),
    ) else {
        return false;
    };
    let (Some(event_class), Some(predicate_sense)) = (
        semantic_event_class(approved.kind),
        approved.predicate_sense,
    ) else {
        return false;
    };
    let Some(spec) = approved_event_predicate_spec(event_class, predicate_sense) else {
        return false;
    };
    let mut observed = Vec::new();
    for claim in claims {
        if !spec.allows_role(claim.relation) || observed.contains(&claim.relation) {
            return false;
        }
        observed.push(claim.relation);
    }
    let has = |relation| claims.iter().any(|claim| claim.relation == relation);
    let valency_complete = spec.required_roles.iter().copied().all(has);
    valency_complete
        && (information.voice != ApprovedEventVoiceIR::Active || has(ApprovedRelationTypeIR::Agent))
}

fn is_semantic_event_kind(kind: DocumentEventPredicateKindIR) -> bool {
    matches!(
        kind,
        DocumentEventPredicateKindIR::Motion
            | DocumentEventPredicateKindIR::Transfer
            | DocumentEventPredicateKindIR::Creation
            | DocumentEventPredicateKindIR::Inspection
            | DocumentEventPredicateKindIR::StateChange
    )
}

fn semantic_event_class(
    kind: DocumentEventPredicateKindIR,
) -> Option<ApprovedEventRealizationClassIR> {
    match kind {
        DocumentEventPredicateKindIR::Motion => Some(ApprovedEventRealizationClassIR::Motion),
        DocumentEventPredicateKindIR::Transfer => Some(ApprovedEventRealizationClassIR::Transfer),
        DocumentEventPredicateKindIR::Creation => Some(ApprovedEventRealizationClassIR::Creation),
        DocumentEventPredicateKindIR::Inspection => {
            Some(ApprovedEventRealizationClassIR::Inspection)
        }
        DocumentEventPredicateKindIR::StateChange => {
            Some(ApprovedEventRealizationClassIR::StateChange)
        }
        DocumentEventPredicateKindIR::HostedOccurrence
        | DocumentEventPredicateKindIR::ScheduledProcess
        | DocumentEventPredicateKindIR::Departure
        | DocumentEventPredicateKindIR::Arrival => None,
    }
}

fn approved_event_predicate(
    response: &ApprovedCompositionalResponseIR,
    subject_node_id: &str,
) -> Option<ApprovedDocumentEventPredicate> {
    response
        .event_realizations
        .iter()
        .find(|realization| realization.subject_node_id == subject_node_id)
        .and_then(|realization| {
            let phase = realization.phase?;
            let kind = match realization.class {
                ApprovedEventRealizationClassIR::HostedEvent => {
                    DocumentEventPredicateKindIR::HostedOccurrence
                }
                ApprovedEventRealizationClassIR::ScheduledProcess => {
                    DocumentEventPredicateKindIR::ScheduledProcess
                }
                ApprovedEventRealizationClassIR::Departure => {
                    DocumentEventPredicateKindIR::Departure
                }
                ApprovedEventRealizationClassIR::Arrival => DocumentEventPredicateKindIR::Arrival,
                ApprovedEventRealizationClassIR::Motion => DocumentEventPredicateKindIR::Motion,
                ApprovedEventRealizationClassIR::Transfer => DocumentEventPredicateKindIR::Transfer,
                ApprovedEventRealizationClassIR::Creation => DocumentEventPredicateKindIR::Creation,
                ApprovedEventRealizationClassIR::Inspection => {
                    DocumentEventPredicateKindIR::Inspection
                }
                ApprovedEventRealizationClassIR::StateChange => {
                    DocumentEventPredicateKindIR::StateChange
                }
            };
            if is_semantic_event_kind(kind)
                && (realization.predicate_sense.is_none()
                    || normalize_event_information(
                        realization.perspective,
                        realization.voice,
                        realization.information_structure.as_ref(),
                    )
                    .is_none())
            {
                return None;
            }
            Some(ApprovedDocumentEventPredicate {
                kind,
                phase,
                perspective: realization.perspective,
                voice: realization.voice,
                information_structure: realization.information_structure.clone(),
                predicate_sense: realization.predicate_sense,
            })
        })
}

fn normalize_event_information(
    perspective: Option<ApprovedEventPerspectiveIR>,
    voice: Option<ApprovedEventVoiceIR>,
    information: Option<&ApprovedEventInformationStructureIR>,
) -> Option<NormalizedEventInformationStructure> {
    match (perspective, voice, information) {
        (Some(perspective), None, None) => {
            let (topic, focus) = match (perspective.voice, perspective.focus) {
                (ApprovedEventVoiceIR::Active, ApprovedEventFocusIR::Agent) => {
                    (None, ApprovedEventInformationRoleIR::Agent)
                }
                (ApprovedEventVoiceIR::Active, ApprovedEventFocusIR::Theme) => (
                    Some(ApprovedEventInformationRoleIR::Theme),
                    ApprovedEventInformationRoleIR::Agent,
                ),
                (ApprovedEventVoiceIR::Active, ApprovedEventFocusIR::Destination) => (
                    Some(ApprovedEventInformationRoleIR::Destination),
                    ApprovedEventInformationRoleIR::Agent,
                ),
                (ApprovedEventVoiceIR::Passive, ApprovedEventFocusIR::Theme) => (
                    Some(ApprovedEventInformationRoleIR::Theme),
                    ApprovedEventInformationRoleIR::Theme,
                ),
                _ => return None,
            };
            Some(NormalizedEventInformationStructure {
                voice: perspective.voice,
                topic,
                focus,
                omitted_roles: Vec::new(),
            })
        }
        (None, Some(voice), Some(information)) => Some(NormalizedEventInformationStructure {
            voice,
            topic: information.topic,
            focus: information.focus,
            omitted_roles: information.omitted_roles.clone(),
        }),
        _ => None,
    }
}

fn event_argument_order(relation: ApprovedRelationTypeIR) -> Option<u8> {
    match relation {
        ApprovedRelationTypeIR::Agent => Some(0),
        ApprovedRelationTypeIR::Patient => Some(1),
        ApprovedRelationTypeIR::Theme => Some(2),
        ApprovedRelationTypeIR::Source => Some(3),
        ApprovedRelationTypeIR::Destination => Some(4),
        ApprovedRelationTypeIR::Target => Some(5),
        ApprovedRelationTypeIR::InitialState => Some(6),
        ApprovedRelationTypeIR::ResultState => Some(7),
        ApprovedRelationTypeIR::Date => Some(8),
        ApprovedRelationTypeIR::Time => Some(9),
        ApprovedRelationTypeIR::Location => Some(10),
        ApprovedRelationTypeIR::Instrument => Some(11),
        ApprovedRelationTypeIR::Manner => Some(12),
        _ => None,
    }
}

fn event_argument_role(relation: ApprovedRelationTypeIR) -> Option<DocumentEventArgumentRoleIR> {
    match relation {
        ApprovedRelationTypeIR::Date => Some(DocumentEventArgumentRoleIR::Date),
        ApprovedRelationTypeIR::Time => Some(DocumentEventArgumentRoleIR::Time),
        ApprovedRelationTypeIR::Location => Some(DocumentEventArgumentRoleIR::Location),
        ApprovedRelationTypeIR::Agent => Some(DocumentEventArgumentRoleIR::Agent),
        ApprovedRelationTypeIR::Patient => Some(DocumentEventArgumentRoleIR::Patient),
        ApprovedRelationTypeIR::Theme => Some(DocumentEventArgumentRoleIR::Theme),
        ApprovedRelationTypeIR::Source => Some(DocumentEventArgumentRoleIR::Source),
        ApprovedRelationTypeIR::Destination => Some(DocumentEventArgumentRoleIR::Destination),
        ApprovedRelationTypeIR::Target => Some(DocumentEventArgumentRoleIR::Target),
        ApprovedRelationTypeIR::Instrument => Some(DocumentEventArgumentRoleIR::Instrument),
        ApprovedRelationTypeIR::Manner => Some(DocumentEventArgumentRoleIR::Manner),
        ApprovedRelationTypeIR::InitialState => Some(DocumentEventArgumentRoleIR::InitialState),
        ApprovedRelationTypeIR::ResultState => Some(DocumentEventArgumentRoleIR::ResultState),
        _ => None,
    }
}

fn plan_clause_fusions(
    response: &ApprovedCompositionalResponseIR,
    sections: &[DocumentResponseSectionIR],
    rhetorical_units: &[DocumentRhetoricalUnitIR],
    event_predicates: &[DocumentEventPredicateIR],
    language: LanguageCodeIR,
) -> Vec<DocumentClauseFusionIR> {
    let mut fusions = Vec::new();
    for section in sections {
        let claims = section
            .claim_ids
            .iter()
            .filter_map(|id| {
                response
                    .claims
                    .iter()
                    .find(|claim| claim.proposition_id == *id)
            })
            .collect::<Vec<_>>();
        let mut fused_clause_count = 1usize;
        for pair in claims.windows(2) {
            let left_move = rhetorical_units
                .iter()
                .find(|unit| unit.proposition_id == pair[0].proposition_id)
                .map(|unit| unit.rhetorical_move);
            let right_move = rhetorical_units
                .iter()
                .find(|unit| unit.proposition_id == pair[1].proposition_id)
                .map(|unit| unit.rhetorical_move);
            let reserved_by_event_predicate = event_predicates.iter().any(|predicate| {
                predicate.arguments.windows(2).any(|arguments| {
                    arguments[0].proposition_id == pair[0].proposition_id
                        && arguments[1].proposition_id == pair[1].proposition_id
                })
            });
            let compatible = !reserved_by_event_predicate
                && pair[0].subject == pair[1].subject
                && left_move == right_move
                && can_coordinate_claim(pair[0], response, language);
            if compatible && fused_clause_count < MAX_FUSED_CLAUSES {
                let fusion_index = fusions.len();
                fusions.push(DocumentClauseFusionIR {
                    fusion_index,
                    left_proposition_id: pair[0].proposition_id.clone(),
                    right_proposition_id: pair[1].proposition_id.clone(),
                    kind: DocumentClauseFusionKindIR::SharedSubjectCoordination,
                    predecessor_fusion_indices: (fusion_index > 0)
                        .then(|| fusion_index - 1)
                        .into_iter()
                        .collect(),
                    semantic_authority: false,
                });
                fused_clause_count += 1;
            } else if pair[0].subject != pair[1].subject || left_move != right_move {
                fused_clause_count = 1;
            }
        }
    }
    fusions
}

fn can_coordinate_claim(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> bool {
    match language {
        LanguageCodeIR::Korean => {
            korean_coordinated_surface(&realize_korean_claim(claim, response)).is_some()
        }
        LanguageCodeIR::English => true,
        _ => false,
    }
}

fn plan_document_sections(
    response: &ApprovedCompositionalResponseIR,
) -> Vec<PlannedDocumentSection> {
    if response.claims.len() <= 2 {
        return vec![PlannedDocumentSection {
            role: role_for(response.discourse_relation),
            claim_ids: response
                .claims
                .iter()
                .map(|claim| claim.proposition_id.clone())
                .collect(),
        }];
    }

    if response.claims.len() >= 20 {
        return plan_long_document_sections(response);
    }

    let mut sections = Vec::<PlannedDocumentSection>::new();
    for claim in &response.claims {
        // Procedure order is carried by visible markers on the adjoining
        // typed steps, so it must not create a standalone empty section.
        if is_directive_procedure_dependency(claim, response) {
            continue;
        }
        let role = role_for_claim(claim, response);
        if let Some(section) = sections.last_mut().filter(|section| section.role == role) {
            section.claim_ids.push(claim.proposition_id.clone());
        } else {
            sections.push(PlannedDocumentSection {
                role,
                claim_ids: vec![claim.proposition_id.clone()],
            });
        }
    }

    while sections.len() > MAX_COMPACT_SECTIONS {
        let merge_index = (0..sections.len() - 1)
            .min_by_key(|index| {
                sections[*index].claim_ids.len() + sections[*index + 1].claim_ids.len()
            })
            .expect("more than one section");
        let right = sections.remove(merge_index + 1);
        let left = &mut sections[merge_index];
        if left.role != right.role {
            left.role = DocumentResponseRoleIR::CoreContent;
        }
        left.claim_ids.extend(right.claim_ids);
    }

    // Bounded merging can turn neighbouring mixed-role sections into the
    // same CoreContent role.  Coalesce those neighbours so long documents do
    // not emit several consecutive headings with the identical label.
    let mut coalesced = Vec::<PlannedDocumentSection>::new();
    for section in sections {
        if let Some(previous) = coalesced
            .last_mut()
            .filter(|previous| previous.role == section.role)
        {
            previous.claim_ids.extend(section.claim_ids);
        } else {
            coalesced.push(section);
        }
    }
    coalesced
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LongDocumentUnitKind {
    EventCurrent,
    EventFuture,
    EventCancelled,
    CauseEffect,
    Action,
    Status,
    OwnerSchedule,
    Other,
}

#[derive(Debug)]
struct LongDocumentUnit {
    kind: LongDocumentUnitKind,
    claim_ids: Vec<String>,
}

/// A heading is a semantic summary of a contiguous document section, not a
/// decorative label.  Keeping this identity separately lets the planner
/// coalesce adjacent sections which would otherwise render under the same
/// heading after event frames are grouped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LongDocumentHeadingSignature {
    TypedCurrent,
    TypedFuture,
    TypedCancelled,
    TypedMixed,
    CauseImpact,
    ActionOwnerSchedule,
    StatusImpact,
    OwnerSchedule,
    StatusLocation,
    Action,
    Status,
    Deadline,
    Fallback(DocumentResponseRoleIR),
}

fn plan_long_document_sections(
    response: &ApprovedCompositionalResponseIR,
) -> Vec<PlannedDocumentSection> {
    let mut units = Vec::<LongDocumentUnit>::new();
    let mut claim_index = 0usize;
    while claim_index < response.claims.len() {
        let claim = &response.claims[claim_index];
        if is_directive_procedure_dependency(claim, response) {
            claim_index += 1;
            continue;
        }
        // An Action wrapper and the event arguments it selects are one
        // semantic realization unit.  Splitting them at a long-document
        // section boundary makes the action appear nominal even though its
        // source supplied a complete predicate, phase, and valency frame.
        // This grouping is based only on typed node identity and contiguous
        // proposition order; it never derives an event from lexical labels.
        if let Some(event_node_id) = typed_action_event_node_id(claim, response) {
            let event_arguments = response.claims[claim_index + 1..]
                .iter()
                .take_while(|candidate| candidate.subject.node_id == event_node_id)
                .collect::<Vec<_>>();
            if !event_arguments.is_empty()
                && approved_event_predicate(response, event_node_id).is_some()
            {
                let mut claim_ids = Vec::with_capacity(event_arguments.len() + 1);
                claim_ids.push(claim.proposition_id.clone());
                claim_ids.extend(
                    event_arguments
                        .iter()
                        .map(|argument| argument.proposition_id.clone()),
                );
                let kind = long_document_unit_kind(event_arguments[0], response);
                units.push(LongDocumentUnit { kind, claim_ids });
                claim_index += event_arguments.len() + 1;
                continue;
            }
        }
        let kind = long_document_unit_kind(claim, response);
        let same_event = units.last().is_some_and(|unit| {
            matches!(
                unit.kind,
                LongDocumentUnitKind::EventCurrent
                    | LongDocumentUnitKind::EventFuture
                    | LongDocumentUnitKind::EventCancelled
            ) && unit.kind == kind
                && unit.claim_ids.first().is_some_and(|first_id| {
                    response
                        .claims
                        .iter()
                        .find(|candidate| candidate.proposition_id == *first_id)
                        .is_some_and(|first| first.subject.node_id == claim.subject.node_id)
                })
        });
        if same_event {
            units
                .last_mut()
                .expect("same event has a preceding unit")
                .claim_ids
                .push(claim.proposition_id.clone());
        } else {
            units.push(LongDocumentUnit {
                kind,
                claim_ids: vec![claim.proposition_id.clone()],
            });
        }
        claim_index += 1;
    }

    // Long-form paragraphs are sized by expected realized sentences rather
    // than raw claim count. A typed event can contain many role claims but
    // realizes as one sentence, so it remains an indivisible unit here.
    let section_count = units.len().div_ceil(8).clamp(3, 4).min(units.len());
    let required_cuts = section_count.saturating_sub(1);
    let mut boundaries = (1..units.len())
        .filter(|index| {
            long_document_boundary_bonus(units[*index - 1].kind, units[*index].kind) >= 60
        })
        .collect::<Vec<_>>();
    if boundaries.len() > required_cuts {
        let mut selected = Vec::with_capacity(required_cuts);
        for ordinal in 1..section_count {
            let ideal = units.len() as f64 * ordinal as f64 / section_count as f64;
            if let Some(boundary) = boundaries
                .iter()
                .copied()
                .filter(|boundary| !selected.contains(boundary))
                .min_by_key(|boundary| ((*boundary as f64 - ideal).abs() * 1_000.0) as usize)
            {
                selected.push(boundary);
            }
        }
        boundaries = selected;
    }
    while boundaries.len() < required_cuts {
        let mut edges = Vec::with_capacity(boundaries.len() + 2);
        edges.push(0usize);
        edges.extend(boundaries.iter().copied());
        edges.push(units.len());
        edges.sort_unstable();
        let (segment_start, segment_end) = edges
            .windows(2)
            .map(|pair| (pair[0], pair[1]))
            .filter(|(start, end)| end - start >= 2)
            .max_by_key(|(start, end)| end - start)
            .expect("a remaining long-document cut has a splittable segment");
        let midpoint = segment_start + (segment_end - segment_start) / 2;
        let boundary = ((segment_start + 1)..segment_end)
            .min_by_key(|candidate| {
                let distance = candidate.abs_diff(midpoint) as i64 * 20;
                distance
                    - long_document_boundary_bonus(
                        units[*candidate - 1].kind,
                        units[*candidate].kind,
                    )
            })
            .expect("a splittable segment has an internal boundary");
        boundaries.push(boundary);
    }
    boundaries.sort_unstable();

    let mut sections = Vec::with_capacity(section_count);
    let mut start = 0usize;
    for end in boundaries.into_iter().chain(std::iter::once(units.len())) {
        let claim_ids = units[start..end]
            .iter()
            .flat_map(|unit| unit.claim_ids.iter().cloned())
            .collect::<Vec<_>>();
        let role = aggregate_long_document_role(&units[start..end]);
        sections.push(PlannedDocumentSection { role, claim_ids });
        start = end;
    }
    coalesce_adjacent_long_sections_by_heading(sections, response)
}

fn coalesce_adjacent_long_sections_by_heading(
    sections: Vec<PlannedDocumentSection>,
    response: &ApprovedCompositionalResponseIR,
) -> Vec<PlannedDocumentSection> {
    let mut coalesced = Vec::<PlannedDocumentSection>::with_capacity(sections.len());
    for section in sections {
        let signature = long_heading_signature_for_ids(&section.claim_ids, response);
        if let Some(previous) = coalesced.last_mut() {
            let previous_signature = long_heading_signature_for_ids(&previous.claim_ids, response);
            if previous_signature == signature {
                previous.claim_ids.extend(section.claim_ids);
                continue;
            }
        }
        coalesced.push(section);
    }
    coalesced
}

fn long_heading_signature_for_ids(
    claim_ids: &[String],
    response: &ApprovedCompositionalResponseIR,
) -> LongDocumentHeadingSignature {
    let claims = claim_ids
        .iter()
        .filter_map(|id| {
            response
                .claims
                .iter()
                .find(|claim| claim.proposition_id == *id)
        })
        .collect::<Vec<_>>();
    long_heading_signature(&claims, response)
}

fn long_heading_signature(
    claims: &[&ApprovedCompositionalClaimIR],
    response: &ApprovedCompositionalResponseIR,
) -> LongDocumentHeadingSignature {
    // Directive event frames describe source-authorized response steps.  They
    // are semantically Actions even though their surface claims are the typed
    // event arguments selected by an Action wrapper.  Label the whole group
    // accordingly instead of exposing the generic EventFrame fallback.
    let directive_event_ids = response
        .claims
        .iter()
        .filter(|claim| {
            claim.relation == ApprovedRelationTypeIR::Action
                && claim.modality == ApprovedModalityIR::Directive
        })
        .filter_map(|claim| typed_action_event_node_id(claim, response))
        .collect::<BTreeSet<_>>();
    if !directive_event_ids.is_empty()
        && claims
            .iter()
            .any(|claim| directive_event_ids.contains(claim.subject.node_id.as_str()))
    {
        return LongDocumentHeadingSignature::Action;
    }

    let typed_event_only = !claims.is_empty()
        && claims
            .iter()
            .all(|claim| is_typed_event_argument(claim, response));
    if typed_event_only {
        let mut has_current = false;
        let mut has_future = false;
        let mut has_cancelled = false;
        for claim in claims {
            if let Some(phase) = response
                .event_realizations
                .iter()
                .find(|event| event.subject_node_id == claim.subject.node_id)
                .and_then(|event| event.phase)
            {
                match phase {
                    ApprovedEventPhaseIR::Completed | ApprovedEventPhaseIR::Ongoing => {
                        has_current = true
                    }
                    ApprovedEventPhaseIR::Planned | ApprovedEventPhaseIR::Scheduled => {
                        has_future = true
                    }
                    ApprovedEventPhaseIR::Cancelled => has_cancelled = true,
                }
            }
        }
        return match (has_current, has_future, has_cancelled) {
            (true, false, false) => LongDocumentHeadingSignature::TypedCurrent,
            (false, true, false) => LongDocumentHeadingSignature::TypedFuture,
            (false, false, true) => LongDocumentHeadingSignature::TypedCancelled,
            _ => LongDocumentHeadingSignature::TypedMixed,
        };
    }

    let has = |relation| claims.iter().any(|claim| claim.relation == relation);
    let cause = has(ApprovedRelationTypeIR::Cause);
    let impact = has(ApprovedRelationTypeIR::Impact);
    let action = has(ApprovedRelationTypeIR::Action);
    let status = has(ApprovedRelationTypeIR::Status)
        || has(ApprovedRelationTypeIR::Confirmed)
        || has(ApprovedRelationTypeIR::Approved);
    let owner = has(ApprovedRelationTypeIR::Owner);
    let deadline = has(ApprovedRelationTypeIR::Deadline)
        || has(ApprovedRelationTypeIR::Date)
        || has(ApprovedRelationTypeIR::Time);
    let location = has(ApprovedRelationTypeIR::Location);

    if cause && impact {
        LongDocumentHeadingSignature::CauseImpact
    } else if action && (owner || deadline) {
        LongDocumentHeadingSignature::ActionOwnerSchedule
    } else if status && impact {
        LongDocumentHeadingSignature::StatusImpact
    } else if owner && deadline {
        LongDocumentHeadingSignature::OwnerSchedule
    } else if status && location {
        LongDocumentHeadingSignature::StatusLocation
    } else if action {
        LongDocumentHeadingSignature::Action
    } else if status {
        LongDocumentHeadingSignature::Status
    } else if deadline {
        LongDocumentHeadingSignature::Deadline
    } else {
        LongDocumentHeadingSignature::Fallback(role_for(response.discourse_relation))
    }
}

fn long_document_unit_kind(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> LongDocumentUnitKind {
    if let Some(event) = response
        .event_realizations
        .iter()
        .find(|event| event.subject_node_id == claim.subject.node_id)
    {
        return event
            .phase
            .map(|phase| match phase {
                ApprovedEventPhaseIR::Completed | ApprovedEventPhaseIR::Ongoing => {
                    LongDocumentUnitKind::EventCurrent
                }
                ApprovedEventPhaseIR::Planned | ApprovedEventPhaseIR::Scheduled => {
                    LongDocumentUnitKind::EventFuture
                }
                ApprovedEventPhaseIR::Cancelled => LongDocumentUnitKind::EventCancelled,
            })
            .unwrap_or(LongDocumentUnitKind::Other);
    }
    match claim.relation {
        ApprovedRelationTypeIR::Cause | ApprovedRelationTypeIR::Impact => {
            LongDocumentUnitKind::CauseEffect
        }
        ApprovedRelationTypeIR::Action => LongDocumentUnitKind::Action,
        ApprovedRelationTypeIR::Owner
        | ApprovedRelationTypeIR::Deadline
        | ApprovedRelationTypeIR::Date
        | ApprovedRelationTypeIR::Time => LongDocumentUnitKind::OwnerSchedule,
        ApprovedRelationTypeIR::Status
        | ApprovedRelationTypeIR::Confirmed
        | ApprovedRelationTypeIR::Approved
        | ApprovedRelationTypeIR::Cancelled
        | ApprovedRelationTypeIR::Registration
        | ApprovedRelationTypeIR::Location
        | ApprovedRelationTypeIR::RoomAvailable
        | ApprovedRelationTypeIR::Entry
        | ApprovedRelationTypeIR::Capacity
        | ApprovedRelationTypeIR::Count
        | ApprovedRelationTypeIR::Quantity => LongDocumentUnitKind::Status,
        _ => LongDocumentUnitKind::Other,
    }
}

fn long_document_boundary_bonus(left: LongDocumentUnitKind, right: LongDocumentUnitKind) -> i64 {
    use LongDocumentUnitKind::*;
    if left == right {
        return 0;
    }
    match (left, right) {
        (EventCurrent, EventFuture)
        | (EventCurrent, EventCancelled)
        | (EventFuture, EventCancelled)
        | (EventCancelled, EventFuture) => 80,
        (EventCurrent | EventFuture | EventCancelled, _)
        | (_, EventCurrent | EventFuture | EventCancelled) => 60,
        _ => 12,
    }
}

fn aggregate_long_document_role(units: &[LongDocumentUnit]) -> DocumentResponseRoleIR {
    use LongDocumentUnitKind::*;
    if units.iter().all(|unit| unit.kind == EventCurrent) {
        return DocumentResponseRoleIR::EventFrame;
    }
    if units.iter().all(|unit| unit.kind == EventFuture) {
        return DocumentResponseRoleIR::Timeline;
    }
    if units.iter().all(|unit| unit.kind == CauseEffect) {
        return DocumentResponseRoleIR::Cause;
    }
    if units.iter().all(|unit| unit.kind == OwnerSchedule) {
        return DocumentResponseRoleIR::Timeline;
    }
    if units
        .iter()
        .all(|unit| matches!(unit.kind, Status | Action))
    {
        return DocumentResponseRoleIR::Status;
    }
    DocumentResponseRoleIR::CoreContent
}

fn role_for_claim(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> DocumentResponseRoleIR {
    if typed_action_event_node_id(claim, response).is_some() {
        return DocumentResponseRoleIR::EventFrame;
    }
    if is_typed_event_argument(claim, response)
        && response.claims.iter().any(|candidate| {
            candidate.proposition_id != claim.proposition_id
                && candidate.subject == claim.subject
                && is_typed_event_argument(candidate, response)
                && candidate.relation != claim.relation
        })
    {
        return match approved_event_predicate(response, &claim.subject.node_id) {
            Some(predicate)
                if response.claims.len() >= 20
                    && matches!(
                        predicate.phase,
                        ApprovedEventPhaseIR::Planned | ApprovedEventPhaseIR::Scheduled
                    ) =>
            {
                DocumentResponseRoleIR::Timeline
            }
            Some(predicate) if is_semantic_event_kind(predicate.kind) => {
                DocumentResponseRoleIR::EventFrame
            }
            _ => DocumentResponseRoleIR::Timeline,
        };
    }
    match claim.relation {
        ApprovedRelationTypeIR::Name => DocumentResponseRoleIR::Identity,
        ApprovedRelationTypeIR::Status
        | ApprovedRelationTypeIR::Cancelled
        | ApprovedRelationTypeIR::Registration
        | ApprovedRelationTypeIR::Confirmed
        | ApprovedRelationTypeIR::Approved => DocumentResponseRoleIR::Status,
        ApprovedRelationTypeIR::Time
        | ApprovedRelationTypeIR::Date
        | ApprovedRelationTypeIR::Duration
        | ApprovedRelationTypeIR::EarlierThan
        | ApprovedRelationTypeIR::Deadline => DocumentResponseRoleIR::Timeline,
        ApprovedRelationTypeIR::Cause => DocumentResponseRoleIR::Cause,
        ApprovedRelationTypeIR::Impact | ApprovedRelationTypeIR::Action => {
            DocumentResponseRoleIR::Status
        }
        ApprovedRelationTypeIR::Owner => DocumentResponseRoleIR::Identity,
        ApprovedRelationTypeIR::Location
        | ApprovedRelationTypeIR::RoomAvailable
        | ApprovedRelationTypeIR::Entry => DocumentResponseRoleIR::PlaceAndAccess,
        ApprovedRelationTypeIR::Capacity
        | ApprovedRelationTypeIR::Count
        | ApprovedRelationTypeIR::Quantity => DocumentResponseRoleIR::Measurement,
        ApprovedRelationTypeIR::Agent
        | ApprovedRelationTypeIR::Patient
        | ApprovedRelationTypeIR::Theme
        | ApprovedRelationTypeIR::Source
        | ApprovedRelationTypeIR::Destination
        | ApprovedRelationTypeIR::Target
        | ApprovedRelationTypeIR::Instrument
        | ApprovedRelationTypeIR::Manner
        | ApprovedRelationTypeIR::InitialState
        | ApprovedRelationTypeIR::ResultState => DocumentResponseRoleIR::EventFrame,
    }
}

fn is_primary_content(kind: DocumentResponseBlockKindIR) -> bool {
    matches!(
        kind,
        DocumentResponseBlockKindIR::Paragraph
            | DocumentResponseBlockKindIR::OrderedList
            | DocumentResponseBlockKindIR::Table
    )
}

fn push_block(
    blocks: &mut Vec<DocumentResponseBlockIR>,
    kind: DocumentResponseBlockKindIR,
    role: DocumentResponseRoleIR,
    claim_ids: Vec<String>,
) -> usize {
    let block_index = blocks.len();
    blocks.push(DocumentResponseBlockIR {
        block_index,
        kind,
        role,
        claim_ids,
        predecessor_indices: (block_index > 0)
            .then(|| block_index - 1)
            .into_iter()
            .collect(),
        semantic_authority: false,
    });
    block_index
}

fn role_for(relation: ApprovedDiscourseRelationIR) -> DocumentResponseRoleIR {
    match relation {
        ApprovedDiscourseRelationIR::Correction => DocumentResponseRoleIR::Correction,
        ApprovedDiscourseRelationIR::Comparison => DocumentResponseRoleIR::Comparison,
        ApprovedDiscourseRelationIR::Cause | ApprovedDiscourseRelationIR::Explanation => {
            DocumentResponseRoleIR::Cause
        }
        ApprovedDiscourseRelationIR::Condition => DocumentResponseRoleIR::Condition,
        ApprovedDiscourseRelationIR::Temporal | ApprovedDiscourseRelationIR::Simultaneous => {
            DocumentResponseRoleIR::Timeline
        }
        _ => DocumentResponseRoleIR::CoreContent,
    }
}

fn render_plan(
    plan: &DocumentResponsePlanIR,
    response: &ApprovedCompositionalResponseIR,
) -> String {
    let mut rendered = Vec::new();
    for block in &plan.blocks {
        let claims = block
            .claim_ids
            .iter()
            .filter_map(|id| {
                response
                    .claims
                    .iter()
                    .find(|claim| claim.proposition_id == *id)
            })
            .collect::<Vec<_>>();
        let surface = match block.kind {
            DocumentResponseBlockKindIR::Lead => {
                lead_surface_for_response(plan.output_language, response.style.register, response)
            }
            DocumentResponseBlockKindIR::SectionHeading => {
                format!(
                    "## {}",
                    heading_surface_for_block(block, plan, response, plan.output_language)
                )
            }
            DocumentResponseBlockKindIR::SubsectionHeading => {
                let ordinal = plan
                    .sections
                    .iter()
                    .position(|section| section.block_indices.contains(&block.block_index))
                    .map(|index| index + 1)
                    .expect("subsection heading belongs to one section");
                format!(
                    "### {ordinal}. {}",
                    heading_surface_for_block(block, plan, response, plan.output_language)
                )
            }
            DocumentResponseBlockKindIR::Paragraph => {
                realize_claim_sequence(&claims, plan, response, plan.output_language)
            }
            DocumentResponseBlockKindIR::OrderedList => action_aware_claim_groups(&claims, plan, response)
                .iter()
                .enumerate()
                .map(|(index, group)| {
                    format!(
                        "{}. {}",
                        index + 1,
                        realize_claim_sequence(group, plan, response, plan.output_language)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            DocumentResponseBlockKindIR::Table => {
                render_claim_table(&claims, plan, response, plan.output_language)
            }
            DocumentResponseBlockKindIR::Chart => {
                render_claim_chart(&claims, plan.output_language).unwrap_or_default()
            }
            DocumentResponseBlockKindIR::Closing => {
                closing_surface(plan.output_language, response.style.register)
            }
        };
        if !surface.is_empty() {
            rendered.push(surface);
        }
    }
    let surface = rendered.join("\n\n");
    if response.speech_act == ApprovedSpeechActIR::Acknowledge {
        format!(
            "{}\n\n{surface}",
            acknowledgement_marker(plan.output_language, response.style.register)
        )
    } else {
        surface
    }
}

fn acknowledgement_marker(language: LanguageCodeIR, register: LanguageRegisterIR) -> &'static str {
    match (language, register) {
        (LanguageCodeIR::Korean, LanguageRegisterIR::Formal) => "확인했습니다.",
        (LanguageCodeIR::Korean, LanguageRegisterIR::Neutral) => "확인했어요.",
        (LanguageCodeIR::Korean, _) => "확인했어.",
        (_, LanguageRegisterIR::Formal) => "Acknowledged.",
        _ => "Got it.",
    }
}

fn lead_surface(language: LanguageCodeIR, register: LanguageRegisterIR) -> String {
    match (language, register) {
        (LanguageCodeIR::Korean, LanguageRegisterIR::Formal) => {
            "핵심 내용을 항목별로 정리했습니다.".into()
        }
        (LanguageCodeIR::Korean, LanguageRegisterIR::Neutral) => {
            "핵심 내용을 항목별로 정리했어요.".into()
        }
        (LanguageCodeIR::Korean, _) => "핵심 내용을 항목별로 정리했어.".into(),
        _ => "The confirmed points are organized by topic below.".into(),
    }
}

fn lead_surface_for_response(
    language: LanguageCodeIR,
    register: LanguageRegisterIR,
    response: &ApprovedCompositionalResponseIR,
) -> String {
    if response.claims.len() < 20 {
        return lead_surface(language, register);
    }
    match (language, register) {
        (LanguageCodeIR::Korean, LanguageRegisterIR::Formal) => {
            "확인된 내용을 흐름에 따라 정리했습니다.".into()
        }
        (LanguageCodeIR::Korean, LanguageRegisterIR::Neutral) => {
            "확인된 내용을 흐름에 따라 정리했어요.".into()
        }
        (LanguageCodeIR::Korean, _) => "확인된 내용을 흐름에 따라 정리했어.".into(),
        _ => "The confirmed information is organized in sequence below.".into(),
    }
}

fn closing_surface(language: LanguageCodeIR, register: LanguageRegisterIR) -> String {
    match (language, register) {
        (LanguageCodeIR::Korean, LanguageRegisterIR::Formal) => {
            "위 내용은 확인된 정보만 반영했습니다.".into()
        }
        (LanguageCodeIR::Korean, LanguageRegisterIR::Neutral) => {
            "위 내용은 확인된 정보만 반영했어요.".into()
        }
        (LanguageCodeIR::Korean, _) => "위 내용은 확인된 정보만 반영했어.".into(),
        _ => "This response stays within the confirmed evidence.".into(),
    }
}

/// Non-factual pragmatic material is accepted only for the speech acts that
/// license it.  It does not stand in for a claim; the response claims still
/// have to be recovered from another clause.
fn allowed_speech_act_marker(surface: &str, response: &ApprovedCompositionalResponseIR) -> bool {
    match response.speech_act {
        ApprovedSpeechActIR::Acknowledge => matches!(
            surface,
            "확인했습니다." | "확인했어요." | "확인했어." | "Acknowledged." | "Got it."
        ),
        ApprovedSpeechActIR::Reassure => matches!(
            surface,
            "걱정하지 않으셔도 됩니다."
                | "걱정하지 않으셔도 돼요."
                | "안심하셔도 됩니다."
                | "안심하셔도 돼요."
                | "안심해도 돼."
                | "괜찮습니다."
                | "괜찮아요."
                | "괜찮아."
        ),
        ApprovedSpeechActIR::Request => {
            matches!(surface, "부탁드립니다." | "부탁드려요." | "부탁해요.")
        }
        ApprovedSpeechActIR::Promise => {
            matches!(surface, "약속드리겠습니다." | "약속할게요." | "약속드려요.")
        }
        ApprovedSpeechActIR::Query => false,
        _ => false,
    }
}

fn heading_surface(role: DocumentResponseRoleIR, language: LanguageCodeIR) -> &'static str {
    match (language, role) {
        (LanguageCodeIR::Korean, DocumentResponseRoleIR::Correction) => "정정 내용",
        (LanguageCodeIR::Korean, DocumentResponseRoleIR::Comparison) => "비교",
        (LanguageCodeIR::Korean, DocumentResponseRoleIR::Cause) => "이유와 설명",
        (LanguageCodeIR::Korean, DocumentResponseRoleIR::Condition) => "조건",
        (LanguageCodeIR::Korean, DocumentResponseRoleIR::Timeline) => "시간 정보",
        (LanguageCodeIR::Korean, DocumentResponseRoleIR::EventFrame) => "사건 구성",
        (LanguageCodeIR::Korean, DocumentResponseRoleIR::Identity) => "대상 정보",
        (LanguageCodeIR::Korean, DocumentResponseRoleIR::Status) => "상태와 결정",
        (LanguageCodeIR::Korean, DocumentResponseRoleIR::PlaceAndAccess) => "장소와 이용",
        (LanguageCodeIR::Korean, DocumentResponseRoleIR::Measurement) => "수량과 규모",
        (LanguageCodeIR::Korean, _) => "핵심 내용",
        (_, DocumentResponseRoleIR::Correction) => "Correction",
        (_, DocumentResponseRoleIR::Comparison) => "Comparison",
        (_, DocumentResponseRoleIR::Cause) => "Reasons and explanation",
        (_, DocumentResponseRoleIR::Condition) => "Conditions",
        (_, DocumentResponseRoleIR::Timeline) => "Time information",
        (_, DocumentResponseRoleIR::EventFrame) => "Event frame",
        (_, DocumentResponseRoleIR::Identity) => "Subject information",
        (_, DocumentResponseRoleIR::Status) => "Status and decisions",
        (_, DocumentResponseRoleIR::PlaceAndAccess) => "Location and access",
        (_, DocumentResponseRoleIR::Measurement) => "Quantities and scale",
        _ => "Key points",
    }
}

fn heading_surface_for_block(
    block: &DocumentResponseBlockIR,
    plan: &DocumentResponsePlanIR,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> &'static str {
    let claim_ids = if block.claim_ids.is_empty()
        && block.kind == DocumentResponseBlockKindIR::SubsectionHeading
    {
        plan.sections
            .iter()
            .find(|section| section.block_indices.contains(&block.block_index))
            .map(|section| section.claim_ids.as_slice())
            .unwrap_or_default()
    } else {
        block.claim_ids.as_slice()
    };
    if claim_ids.is_empty() {
        return heading_surface(block.role, language);
    }

    let claims = claim_ids
        .iter()
        .filter_map(|id| {
            response
                .claims
                .iter()
                .find(|claim| claim.proposition_id == *id)
        })
        .collect::<Vec<_>>();
    // Compact documents do not use the full long-document heading planner,
    // but a complete directive Action→Event bundle is still a response step,
    // never a generic "event frame".  This check is structural and applies
    // without inspecting Korean labels.
    if response.claims.len() < 20 {
        let directive_event_ids = response
            .claims
            .iter()
            .filter(|claim| {
                claim.relation == ApprovedRelationTypeIR::Action
                    && claim.modality == ApprovedModalityIR::Directive
            })
            .filter_map(|claim| typed_action_event_node_id(claim, response))
            .collect::<BTreeSet<_>>();
        if claims
            .iter()
            .any(|claim| directive_event_ids.contains(claim.subject.node_id.as_str()))
        {
            return heading_surface_for_long_signature(LongDocumentHeadingSignature::Action, language);
        }
        return heading_surface(block.role, language);
    }
    heading_surface_for_long_signature(long_heading_signature(&claims, response), language)
}

fn heading_surface_for_long_signature(
    signature: LongDocumentHeadingSignature,
    language: LanguageCodeIR,
) -> &'static str {
    match (language, signature) {
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::TypedCurrent) => "진행 및 완료 사항",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::TypedFuture) => "향후 실행 계획",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::TypedCancelled) => "취소 사항",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::TypedMixed) => "실행 현황과 계획",
        (_, LongDocumentHeadingSignature::TypedCurrent) => "Work completed and in progress",
        (_, LongDocumentHeadingSignature::TypedFuture) => "Planned work",
        (_, LongDocumentHeadingSignature::TypedCancelled) => "Cancelled work",
        (_, LongDocumentHeadingSignature::TypedMixed) => "Work status and plans",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::CauseImpact) => "원인과 영향",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::ActionOwnerSchedule) => "대응 조치와 담당 일정",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::StatusImpact) => "현재 상태와 영향",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::OwnerSchedule) => "담당과 다음 일정",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::StatusLocation) => "운영 상태와 위치",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::Action) => "대응 조치",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::Status) => "현재 상태",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::Deadline) => "향후 일정",
        (LanguageCodeIR::Korean, LongDocumentHeadingSignature::Fallback(role)) => heading_surface(role, language),
        (_, LongDocumentHeadingSignature::CauseImpact) => "Causes and effects",
        (_, LongDocumentHeadingSignature::ActionOwnerSchedule) => "Actions, owners, and schedule",
        (_, LongDocumentHeadingSignature::StatusImpact) => "Current status and effects",
        (_, LongDocumentHeadingSignature::OwnerSchedule) => "Owners and next dates",
        (_, LongDocumentHeadingSignature::StatusLocation) => "Operating status and location",
        (_, LongDocumentHeadingSignature::Action) => "Actions",
        (_, LongDocumentHeadingSignature::Status) => "Current status",
        (_, LongDocumentHeadingSignature::Deadline) => "Upcoming dates",
        (_, LongDocumentHeadingSignature::Fallback(role)) => heading_surface(role, language),
    }
}

fn render_claim_table(
    claims: &[&ApprovedCompositionalClaimIR],
    plan: &DocumentResponsePlanIR,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> String {
    let header = match language {
        LanguageCodeIR::Korean => "| 대상 | 항목 | 값 |\n|---|---|---|",
        _ => "| Subject | Field | Value |\n|---|---|---|",
    };
    let rows = claims
        .iter()
        .map(|claim| {
            format!(
                "| {} | {} | {} |",
                escape_cell(&claim.subject.canonical_lexical_label),
                relation_label(claim.relation, language),
                escape_cell(&display_value(
                    &claim.value,
                    language,
                    response.style.register
                ))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let sentences = realize_claim_sequence(claims, plan, response, language);
    format!("{sentences}\n\n{header}\n{rows}")
}

fn render_claim_chart(
    claims: &[&ApprovedCompositionalClaimIR],
    language: LanguageCodeIR,
) -> Option<String> {
    let values = claims
        .iter()
        .map(|claim| numeric_value(&claim.value))
        .collect::<Option<Vec<_>>>()?;
    let labels = claims
        .iter()
        .map(|claim| {
            format!(
                "\"{}\"",
                mermaid_text(&claim.subject.canonical_lexical_label)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let values = values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let title = match language {
        LanguageCodeIR::Korean => "값 비교",
        _ => "Value comparison",
    };
    Some(format!(
        "### {title}\n\n```mermaid\nxychart-beta\n    x-axis [{labels}]\n    y-axis \"{}\"\n    bar [{values}]\n```",
        relation_label(claims[0].relation, language)
    ))
}

fn chart_claims<'a>(
    response: &'a ApprovedCompositionalResponseIR,
    claim_ids: &[String],
) -> Option<Vec<&'a ApprovedCompositionalClaimIR>> {
    let claims = claim_ids
        .iter()
        .map(|id| {
            response
                .claims
                .iter()
                .find(|claim| claim.proposition_id == *id)
        })
        .collect::<Option<Vec<_>>>()?;
    let relation = claims.first()?.relation;
    (claims.len() >= 2
        && claims.iter().all(|claim| {
            claim.relation == relation && claim.polarity && numeric_value(&claim.value).is_some()
        }))
    .then_some(claims)
}

fn claims_share_subject(response: &ApprovedCompositionalResponseIR, claim_ids: &[String]) -> bool {
    let mut claims = claim_ids.iter().filter_map(|id| {
        response
            .claims
            .iter()
            .find(|claim| claim.proposition_id == *id)
    });
    let Some(first) = claims.next() else {
        return false;
    };
    claims.all(|claim| claim.subject == first.subject)
}

/// Keeps a typed action relation and the event arguments it names in one
/// document unit.  The grouping is structural: it uses proposition IDs in the
/// validated event predicate and never depends on Korean strings.
fn action_aware_claim_groups<'a>(
    claims: &[&'a ApprovedCompositionalClaimIR],
    plan: &DocumentResponsePlanIR,
    response: &ApprovedCompositionalResponseIR,
) -> Vec<Vec<&'a ApprovedCompositionalClaimIR>> {
    let mut groups = Vec::new();
    let mut index = 0usize;
    while index < claims.len() {
        let claim = claims[index];
        if is_directive_procedure_dependency(claim, response) {
            index += 1;
            continue;
        }
        if let Some(predicate) = action_linked_event_predicate(claim, plan, response) {
            let ids = predicate
                .arguments
                .iter()
                .map(|argument| argument.proposition_id.as_str())
                .collect::<Vec<_>>();
            if claims[index + 1..]
                .iter()
                .zip(&ids)
                .all(|(candidate, id)| candidate.proposition_id == *id)
                && claims.len().saturating_sub(index + 1) >= ids.len()
            {
                let mut group = Vec::with_capacity(ids.len() + 1);
                group.push(claim);
                group.extend_from_slice(&claims[index + 1..index + 1 + ids.len()]);
                groups.push(group);
                index += ids.len() + 1;
                continue;
            }
        }
        let mut group = vec![claim];
        index += 1;
        while index < claims.len() && claims[index].subject == claim.subject {
            group.push(claims[index]);
            index += 1;
        }
        groups.push(group);
    }
    groups
}



fn realize_claim_sequence(
    claims: &[&ApprovedCompositionalClaimIR],
    plan: &DocumentResponsePlanIR,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> String {
    let mut previous_subject = None::<&ApprovedLexicalNodeIR>;
    let mut rendered = Vec::<(String, String)>::new();
    let mut consumed_typed_action_argument_ids = BTreeSet::<String>::new();
    let mut claim_index = 0usize;
    while claim_index < claims.len() {
        let claim = claims[claim_index];
        if is_directive_procedure_dependency(claim, response) {
            claim_index += 1;
            continue;
        }
        if consumed_typed_action_argument_ids.contains(&claim.proposition_id) {
            claim_index += 1;
            continue;
        }
        let first_response_claim = plan
            .rhetorical_units
            .first()
            .is_some_and(|first| first.proposition_id == claim.proposition_id);
        if let Some(predicate) = action_linked_event_predicate(claim, plan, response) {
            let predicate_claims = event_predicate_claims(predicate, response)
                .expect("validated typed action event must reference approved claims");
            let event_surface = realize_event_predicate(predicate, &predicate_claims, response, language)
                .expect("validated typed action event must be realizable");
            let surface = match language {
                LanguageCodeIR::Korean => {
                    korean_typed_action_event_clause(&event_surface)
                        .expect("typed action event must have a Korean clause")
                }
                LanguageCodeIR::English => format!(
                    "For {}, {}",
                    claim.subject.canonical_lexical_label.trim(),
                    event_surface.trim()
                ),
                _ => unreachable!("document language was validated"),
            };
            let surface = procedure_sequence_marker(claim, response, language)
                .map(|marker| format!("{marker} {surface}"))
                .unwrap_or(surface);
            consumed_typed_action_argument_ids.extend(
                predicate_claims
                    .iter()
                    .map(|argument| argument.proposition_id.clone()),
            );
            let surface = match language {
                LanguageCodeIR::Korean if !surface.ends_with('.') => format!("{surface}."),
                _ => surface,
            };
            rendered.push((claim.proposition_id.clone(), surface));
            previous_subject = Some(&claim.subject);
            claim_index += 1;
            continue;
        }
        if let Some(predicate) = plan.event_predicates.iter().find(|predicate| {
            predicate
                .arguments
                .first()
                .is_some_and(|argument| argument.proposition_id == claim.proposition_id)
                && claims[claim_index..]
                    .iter()
                    .zip(&predicate.arguments)
                    .all(|(claim, argument)| claim.proposition_id == argument.proposition_id)
                && claims.len() - claim_index >= predicate.arguments.len()
        }) {
            let predicate_claims = event_predicate_claims(predicate, response)
                .expect("validated event predicate must reference approved claims");
            let mut surface =
                realize_event_predicate(predicate, &predicate_claims, response, language)
                    .expect("validated event predicate must be realizable");
            if let Some(unit) = plan
                .rhetorical_units
                .iter()
                .find(|unit| unit.proposition_id == claim.proposition_id)
            {
                let first_claim = plan
                    .rhetorical_units
                    .first()
                    .is_some_and(|first| first.proposition_id == claim.proposition_id);
                if let Some(prefix) = rhetorical_prefix(
                    unit.rhetorical_move,
                    response.discourse_relation,
                    first_claim,
                    language,
                ) {
                    surface = apply_rhetorical_prefix(&surface, prefix, language);
                }
            }
            let last = predicate_claims.last().expect("at least two arguments");
            rendered.push((last.proposition_id.clone(), surface));
            previous_subject = Some(&last.subject);
            claim_index += predicate.arguments.len();
            continue;
        }
        let mut surface = if previous_subject.is_some_and(|subject| subject == &claim.subject) {
            realize_followup_claim(claim, response, language)
        } else {
            realize_claim(claim, response, language)
        };
        if response.speech_act == ApprovedSpeechActIR::Reassure
            && language == LanguageCodeIR::Korean
        {
            let marker = korean_reassurance_marker(response.style.register);
            let prefix = format!("{marker} ");
            surface = surface
                .strip_prefix(&prefix)
                .unwrap_or(&surface)
                .to_string();
            if first_response_claim {
                surface = format!("{marker} {surface}");
            }
        }
        if let Some(unit) = plan
            .rhetorical_units
            .iter()
            .find(|unit| unit.proposition_id == claim.proposition_id)
        {
            let first_claim = plan
                .rhetorical_units
                .first()
                .is_some_and(|first| first.proposition_id == claim.proposition_id);
            if let Some(prefix) = rhetorical_prefix(
                unit.rhetorical_move,
                response.discourse_relation,
                first_claim,
                language,
            ) {
                surface = apply_rhetorical_prefix(&surface, prefix, language);
            }
        }
        if let Some((left_id, left_surface)) = rendered.last().cloned() {
            if plan.clause_fusions.iter().any(|fusion| {
                fusion.left_proposition_id == left_id
                    && fusion.right_proposition_id == claim.proposition_id
            }) {
                let fused = fuse_clause_surfaces(&left_surface, &surface, language)
                    .expect("validated clause fusion must be realizable");
                rendered.pop();
                rendered.push((claim.proposition_id.clone(), fused));
            } else {
                rendered.push((claim.proposition_id.clone(), surface));
            }
        } else {
            rendered.push((claim.proposition_id.clone(), surface));
        }
        previous_subject = Some(&claim.subject);
        claim_index += 1;
    }
    let surfaces = rendered
        .into_iter()
        .map(|(_, surface)| surface)
        .collect::<Vec<_>>();
    if response.claims.len() >= 20 && surfaces.len() > 7 {
        // Keep long prose readable without leaving a one-sentence tail.  A
        // fixed six-sentence chunk produced 6+1 and 6+6+6+1 paragraphs for
        // otherwise coherent sections.  Distribute sentences evenly while
        // preserving their approved order and section membership.
        let paragraph_count = (surfaces.len() + 5) / 6;
        let base_size = surfaces.len() / paragraph_count;
        let larger_paragraphs = surfaces.len() % paragraph_count;
        let mut paragraphs = Vec::with_capacity(paragraph_count);
        let mut start = 0usize;
        for paragraph_index in 0..paragraph_count {
            let paragraph_size = base_size + usize::from(paragraph_index < larger_paragraphs);
            let end = start + paragraph_size;
            paragraphs.push(surfaces[start..end].join(" "));
            start = end;
        }
        paragraphs.join("\n\n")
    } else {
        surfaces.join(" ")
    }
}

fn event_predicate_claims<'a>(
    predicate: &DocumentEventPredicateIR,
    response: &'a ApprovedCompositionalResponseIR,
) -> Option<Vec<&'a ApprovedCompositionalClaimIR>> {
    predicate
        .arguments
        .iter()
        .map(|argument| {
            response
                .claims
                .iter()
                .find(|claim| claim.proposition_id == argument.proposition_id)
        })
        .collect()
}

fn action_linked_event_predicate<'a>(
    claim: &ApprovedCompositionalClaimIR,
    plan: &'a DocumentResponsePlanIR,
    response: &ApprovedCompositionalResponseIR,
) -> Option<&'a DocumentEventPredicateIR> {
    let event_node_id = typed_action_event_node_id(claim, response)?;
    plan.event_predicates.iter().find(|predicate| {
        predicate.subject_node_id == event_node_id
            && predicate.predicate_sense.is_some()
            && event_predicate_claims(predicate, response).is_some()
    })
}

fn realize_event_predicate(
    predicate: &DocumentEventPredicateIR,
    claims: &[&ApprovedCompositionalClaimIR],
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> Option<String> {
    if claims.is_empty()
        || claims.len() > MAX_EVENT_ARGUMENTS
        || claims
            .first()
            .is_none_or(|claim| claim.subject.node_id != predicate.subject_node_id)
        || approved_event_predicate(response, &predicate.subject_node_id)
            != Some(ApprovedDocumentEventPredicate {
                kind: predicate.kind,
                phase: predicate.phase,
                predicate_sense: predicate.predicate_sense,
                perspective: predicate.perspective,
                voice: predicate.voice,
                information_structure: predicate.information_structure.clone(),
            })
        || predicate
            .arguments
            .iter()
            .zip(claims)
            .any(|(argument, claim)| {
                claim.proposition_id != argument.proposition_id
                    || event_argument_role(claim.relation) != Some(argument.role)
                    || !is_typed_event_argument(claim, response)
            })
    {
        return None;
    }
    if is_semantic_event_kind(predicate.kind) {
        return realize_semantic_event_predicate(predicate, claims, response, language);
    }
    let subject = claims.first()?.subject.canonical_lexical_label.trim();
    let mut date = None;
    let mut time = None;
    let mut location = None;
    for (argument, claim) in predicate.arguments.iter().zip(claims) {
        let value = display_value(&claim.value, language, response.style.register);
        match argument.role {
            DocumentEventArgumentRoleIR::Date => date = Some(value),
            DocumentEventArgumentRoleIR::Time => time = Some(value),
            DocumentEventArgumentRoleIR::Location => location = Some(value),
            _ => return None,
        }
    }
    match language {
        LanguageCodeIR::Korean => {
            let mut circumstances = Vec::new();
            match (date, time) {
                (Some(date), Some(time)) => circumstances.push(format!("{date} {time}에")),
                (Some(date), None) => circumstances.push(format!("{date}에")),
                (None, Some(time)) => circumstances.push(format!("{time}에")),
                (None, None) => {}
            }
            if let Some(location) = location {
                let particle = match predicate.kind {
                    DocumentEventPredicateKindIR::Arrival => "에",
                    DocumentEventPredicateKindIR::HostedOccurrence
                    | DocumentEventPredicateKindIR::ScheduledProcess
                    | DocumentEventPredicateKindIR::Departure => "에서",
                    DocumentEventPredicateKindIR::Motion
                    | DocumentEventPredicateKindIR::Transfer
                    | DocumentEventPredicateKindIR::Creation
                    | DocumentEventPredicateKindIR::Inspection
                    | DocumentEventPredicateKindIR::StateChange => return None,
                };
                circumstances.push(format!("{location}{particle}"));
            }
            let topic = object_particle(subject, "은", "는").0;
            let ending =
                korean_event_predicate(predicate.kind, predicate.phase, response.style.register)?;
            Some(format!(
                "{subject}{topic} {} {ending}.",
                circumstances.join(" ")
            ))
        }
        LanguageCodeIR::English => {
            let mut circumstances = Vec::new();
            let transit = matches!(
                predicate.kind,
                DocumentEventPredicateKindIR::Departure | DocumentEventPredicateKindIR::Arrival
            );
            if transit {
                if let Some(location) = &location {
                    let preposition = if predicate.kind == DocumentEventPredicateKindIR::Departure {
                        "from"
                    } else {
                        "at"
                    };
                    circumstances.push(format!("{preposition} {location}"));
                }
            }
            if let Some(date) = date {
                circumstances.push(format!("on {date}"));
            }
            if let Some(time) = time {
                circumstances.push(format!("at {time}"));
            }
            if !transit {
                if let Some(location) = location {
                    circumstances.push(format!("in {location}"));
                }
            }
            let predicate_surface = english_event_predicate(predicate.kind, predicate.phase)?;
            Some(format!(
                "{subject} {predicate_surface} {}.",
                circumstances.join(" ")
            ))
        }
        _ => None,
    }
}

fn realize_semantic_event_predicate(
    predicate: &DocumentEventPredicateIR,
    claims: &[&ApprovedCompositionalClaimIR],
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> Option<String> {
    let action_modality = response.claims.iter().find_map(|claim| {
        typed_action_event_node_id(claim, response)
            .is_some_and(|node_id| node_id == predicate.subject_node_id)
            .then_some(claim.modality)
    });
    let directive = action_modality == Some(ApprovedModalityIR::Directive);
    let commissive = action_modality == Some(ApprovedModalityIR::Commissive);
    let information = normalize_event_information(
        predicate.perspective,
        predicate.voice,
        predicate.information_structure.as_ref(),
    )?;
    match information.voice {
        ApprovedEventVoiceIR::Active
            if !matches!(
                information.focus,
                ApprovedEventInformationRoleIR::Agent | ApprovedEventInformationRoleIR::Event
            ) =>
        {
            return None;
        }
        ApprovedEventVoiceIR::Passive
            if information.focus != ApprovedEventInformationRoleIR::Theme =>
        {
            return None;
        }
        _ => {}
    }
    let predicate_sense = predicate.predicate_sense?;
    let role_claim = |role| {
        predicate
            .arguments
            .iter()
            .zip(claims)
            .find_map(|(argument, claim)| (argument.role == role).then_some(*claim))
    };
    let displayed = |role| {
        role_claim(role).map(|claim| display_value(&claim.value, language, response.style.register))
    };
    let agent = displayed(DocumentEventArgumentRoleIR::Agent);
    let patient = displayed(DocumentEventArgumentRoleIR::Patient);
    let theme = displayed(DocumentEventArgumentRoleIR::Theme)?;
    let source = displayed(DocumentEventArgumentRoleIR::Source);
    let destination = displayed(DocumentEventArgumentRoleIR::Destination);
    let target = displayed(DocumentEventArgumentRoleIR::Target);
    let initial_state = displayed(DocumentEventArgumentRoleIR::InitialState);
    let result_state = displayed(DocumentEventArgumentRoleIR::ResultState);
    let date = displayed(DocumentEventArgumentRoleIR::Date);
    let time = displayed(DocumentEventArgumentRoleIR::Time);
    let location = displayed(DocumentEventArgumentRoleIR::Location);
    let instrument = displayed(DocumentEventArgumentRoleIR::Instrument);
    let manner = displayed(DocumentEventArgumentRoleIR::Manner);

    match language {
        LanguageCodeIR::Korean => {
            // Withdraw is intransitive motion: Theme is the moving party.
            // Korean therefore realizes it with an active Theme predicate
            // even when a generic event perspective is passive.  A generic
            // passive nominal frame would yield the ungrammatical
            // “작업자가 후퇴될 예정”.
            if predicate_sense == ApprovedEventPredicateSenseIR::Withdraw {
                let mut phrases = vec![with_particle(&theme, "은", "는")];
                match (date, time) {
                    (Some(date), Some(time)) => phrases.push(format!("{date} {time}에")),
                    (Some(date), None) => phrases.push(format!("{date}에")),
                    (None, Some(time)) => phrases.push(format!("{time}에")),
                    (None, None) => {}
                }
                match (location, source) {
                    (Some(location), Some(source)) => {
                        phrases.push(format!("{location}의 {source}에서"))
                    }
                    (Some(location), None) => phrases.push(format!("{location}에서")),
                    (None, Some(source)) => phrases.push(format!("{source}에서")),
                    (None, None) => {}
                }
                if directive || commissive {
                    let ending = match (directive, response.style.register) {
                        (true, LanguageRegisterIR::Formal) => "후퇴해야 합니다",
                        (true, LanguageRegisterIR::Neutral) => "후퇴해야 해요",
                        (true, LanguageRegisterIR::Informal | LanguageRegisterIR::Internet) => {
                            "후퇴해야 해"
                        }
                        (false, LanguageRegisterIR::Formal) => "후퇴하겠습니다",
                        (false, LanguageRegisterIR::Neutral) => "후퇴할게요",
                        (false, LanguageRegisterIR::Informal | LanguageRegisterIR::Internet) => {
                            "후퇴할게"
                        }
                    };
                    return Some(format!("{} {ending}.", phrases.join(" ")));
                }
                let ending = korean_semantic_event_predicate(
                    predicate_sense,
                    predicate.phase,
                    ApprovedEventVoiceIR::Active,
                    response.style.register,
                );
                return Some(format!("{} {ending}.", phrases.join(" ")));
            }
            // Fill has an asymmetric Korean valency: Theme is the filling
            // material, while Target is the gap/container affected by the
            // event.  Treating Theme as the ordinary object produces the
            // semantically recoverable but unnatural “충전재를 구멍에 충전”.
            // The role reversal below is part of the predicate codec, not a
            // source-label or style-specific rewrite.
            if predicate_sense == ApprovedEventPredicateSenseIR::Fill {
                let target = target.as_deref()?;
                let mut phrases = Vec::new();
                match (date, time) {
                    (Some(date), Some(time)) => phrases.push(format!("{date} {time}에")),
                    (Some(date), None) => phrases.push(format!("{date}에")),
                    (None, Some(time)) => phrases.push(format!("{time}에")),
                    (None, None) => {}
                }
                match (location, source) {
                    (Some(location), Some(source)) => {
                        phrases.push(format!("{location}의 {source}에서"))
                    }
                    (Some(location), None) => phrases.push(format!("{location}에서")),
                    (None, Some(source)) => phrases.push(format!("{source}에서")),
                    (None, None) => {}
                }
                if directive || commissive {
                    phrases.push(with_particle(target, "을", "를"));
                    phrases.push(with_directional_particle(&theme));
                    let ending = match (directive, response.style.register) {
                        (true, LanguageRegisterIR::Formal) => "메워야 합니다",
                        (true, LanguageRegisterIR::Neutral) => "메워야 해요",
                        (true, LanguageRegisterIR::Informal | LanguageRegisterIR::Internet) => {
                            "메워야 해"
                        }
                        (false, LanguageRegisterIR::Formal) => "메우겠습니다",
                        (false, LanguageRegisterIR::Neutral) => "메울게요",
                        (false, LanguageRegisterIR::Informal | LanguageRegisterIR::Internet) => {
                            "메울게"
                        }
                    };
                    return Some(format!("{} {ending}.", phrases.join(" ")));
                }
                match information.voice {
                    ApprovedEventVoiceIR::Active => {
                        phrases.push(with_particle(agent.as_deref()?, "이", "가"));
                        phrases.push(with_particle(target, "을", "를"));
                        phrases.push(with_directional_particle(&theme));
                    }
                    ApprovedEventVoiceIR::Passive => {
                        phrases.push(with_particle(target, "이", "가"));
                        phrases.push(with_directional_particle(&theme));
                        if let Some(agent) = agent {
                            phrases.push(format!("{agent}에 의해"));
                        }
                    }
                }
                let ending = korean_semantic_event_predicate(
                    predicate_sense,
                    predicate.phase,
                    information.voice,
                    response.style.register,
                );
                return Some(format!("{} {ending}.", phrases.join(" ")));
            }
            if directive || commissive {
                let mut phrases = Vec::new();
                match (date, time) {
                    (Some(date), Some(time)) => phrases.push(format!("{date} {time}에")),
                    (Some(date), None) => phrases.push(format!("{date}에")),
                    (None, Some(time)) => phrases.push(format!("{time}에")),
                    (None, None) => {}
                }
                match (location, source) {
                    (Some(location), Some(source)) => phrases.push(format!("{location}의 {source}에서")),
                    (Some(location), None) => phrases.push(format!("{location}에서")),
                    (None, Some(source)) => phrases.push(format!("{source}에서")),
                    (None, None) => {}
                }
                if let Some(patient) = patient {
                    if predicate_sense == ApprovedEventPredicateSenseIR::Share {
                        phrases.push(with_particle(&patient, "과", "와"));
                    } else {
                        phrases.push(format!("{patient}에게"));
                    }
                }
                if let Some(instrument) = instrument {
                    phrases.push(format!("{} 이용해", with_particle(&instrument, "을", "를")));
                }
                if let Some(manner) = manner {
                    phrases.push(korean_manner_phrase(&manner));
                }
                // Procedural Korean normally introduces a method before the
                // affected object: “표본 검사를 이용해 감염 수준을 기록…”.
                // This is a role-order rule shared by every directive frame.
                phrases.push(with_particle(&theme, "을", "를"));
                if let Some(destination) = destination {
                    phrases.push(with_directional_particle(&destination));
                }
                if let Some(target) = target {
                    phrases.push(format!("{target}에"));
                }
                if let Some(result_state) = result_state {
                    phrases.push(with_directional_particle(&result_state));
                }
                let nominal = event_language_codec(predicate_sense).korean_nominal;
                let ending = match (directive, response.style.register) {
                    (true, LanguageRegisterIR::Formal) => format!("{nominal}해야 합니다"),
                    (true, LanguageRegisterIR::Neutral) => format!("{nominal}해야 해요"),
                    (true, LanguageRegisterIR::Informal | LanguageRegisterIR::Internet) => {
                        format!("{nominal}해야 해")
                    }
                    (false, LanguageRegisterIR::Formal) => format!("{nominal}하겠습니다"),
                    (false, LanguageRegisterIR::Neutral) => format!("{nominal}할게요"),
                    (false, LanguageRegisterIR::Informal | LanguageRegisterIR::Internet) => {
                        format!("{nominal}할게")
                    }
                };
                return Some(format!("{} {ending}.", phrases.join(" ")));
            }
            let mut phrases = Vec::new();
            match information.topic {
                Some(ApprovedEventInformationRoleIR::Agent) => {
                    phrases.push(with_particle(agent.as_deref()?, "은", "는"));
                }
                Some(ApprovedEventInformationRoleIR::Theme) => {
                    phrases.push(with_particle(&theme, "은", "는"));
                }
                Some(ApprovedEventInformationRoleIR::Patient) => {
                    phrases.push(format!("{}에게는", patient.as_deref()?));
                }
                // Korean normally leaves a source adjunct in its canonical
                // predicate slot unless the discourse explicitly licenses a
                // contrastive frame. Semantic salience alone must not force
                // marked clause-initial scrambling.
                Some(ApprovedEventInformationRoleIR::Source) => {}
                Some(ApprovedEventInformationRoleIR::Destination) => {
                    phrases.push(format!("{}에는", destination.as_deref()?));
                }
                Some(_) => return None,
                None => {}
            }
            match information.voice {
                ApprovedEventVoiceIR::Active => {
                    if information.topic != Some(ApprovedEventInformationRoleIR::Agent) {
                        phrases.push(with_particle(agent.as_deref()?, "이", "가"));
                    }
                }
                ApprovedEventVoiceIR::Passive => {
                    if information.topic != Some(ApprovedEventInformationRoleIR::Theme) {
                        phrases.push(with_particle(&theme, "이", "가"));
                    }
                    if !information
                        .omitted_roles
                        .contains(&ApprovedRelationTypeIR::Agent)
                    {
                        if let Some(agent) = &agent {
                            phrases.push(format!("{agent}에 의해"));
                        }
                    }
                }
            }
            // Unmarked Korean event clauses normally establish time and scene
            // before introducing the affected object. Keeping this ordering
            // structural avoids domain-specific word-order templates.
            match (date, time) {
                (Some(date), Some(time)) => phrases.push(format!("{date} {time}에")),
                (Some(date), None) => phrases.push(format!("{date}에")),
                (None, Some(time)) => phrases.push(format!("{time}에")),
                (None, None) => {}
            }
            match (location, source) {
                (Some(location), Some(source)) => {
                    phrases.push(format!("{location}의 {source}에서"));
                }
                (Some(location), None) => phrases.push(format!("{location}에서")),
                (None, Some(source)) => phrases.push(format!("{source}에서")),
                (None, None) => {}
            }
            if information.topic != Some(ApprovedEventInformationRoleIR::Patient) {
                if let Some(patient) = patient {
                    if predicate_sense == ApprovedEventPredicateSenseIR::Share {
                        phrases.push(with_particle(&patient, "과", "와"));
                    } else {
                        phrases.push(format!("{patient}에게"));
                    }
                }
            }
            if information.topic != Some(ApprovedEventInformationRoleIR::Theme)
                && information.voice == ApprovedEventVoiceIR::Active
            {
                phrases.push(with_particle(&theme, "을", "를"));
            }
            if let Some(initial_state) = initial_state {
                phrases.push(format!("{initial_state}에서"));
            }
            if let Some(instrument) = instrument {
                if destination.is_some() || result_state.is_some() {
                    phrases.push(format!("{} 이용해", with_particle(&instrument, "을", "를")));
                } else {
                    phrases.push(with_directional_particle(&instrument));
                }
            }
            if let Some(manner) = manner {
                phrases.push(korean_manner_phrase(&manner));
            }
            // Korean keeps a directional/result complement close to the
            // finite predicate, especially when instrument and manner are
            // long.  Place those adjuncts first, then close the clause with
            // Destination/Target/ResultState rather than separating the goal
            // from its verb.
            if information.topic != Some(ApprovedEventInformationRoleIR::Destination) {
                if let Some(destination) = destination {
                    phrases.push(with_directional_particle(&destination));
                }
            }
            if let Some(target) = target {
                phrases.push(match predicate_sense {
                    ApprovedEventPredicateSenseIR::Expand => format!("{target}까지"),
                    // Compare and Connect are symmetric at the surface
                    // boundary: their second semantic operand is a
                    // counterpart, not a destination. Keep the role
                    // distinction in the trace while using Korean's
                    // comitative particle.
                    ApprovedEventPredicateSenseIR::Compare
                    | ApprovedEventPredicateSenseIR::Connect => {
                        with_particle(&target, "과", "와")
                    }
                    _ => format!("{target}에"),
                });
            }
            if let Some(result_state) = result_state {
                phrases.push(with_directional_particle(&result_state));
            }
            let ending = korean_semantic_event_predicate(
                predicate_sense,
                predicate.phase,
                information.voice,
                response.style.register,
            );
            Some(format!("{} {ending}.", phrases.join(" ")))
        }
        LanguageCodeIR::English => {
            let mut circumstances = Vec::new();
            if let Some(patient) = &patient {
                let preposition = if matches!(
                    predicate_sense,
                    ApprovedEventPredicateSenseIR::Deliver | ApprovedEventPredicateSenseIR::Give
                ) {
                    "to"
                } else {
                    "for"
                };
                circumstances.push(format!("{preposition} {patient}"));
            }
            if let Some(source) = &source {
                circumstances.push(format!("from {source}"));
            }
            if let Some(destination) = &destination {
                circumstances.push(format!("to {destination}"));
            }
            if let Some(target) = &target {
                circumstances.push(format!("to {target}"));
            }
            if let Some(initial_state) = initial_state {
                circumstances.push(format!("from {initial_state}"));
            }
            if let Some(result_state) = result_state {
                circumstances.push(format!("to {result_state}"));
            }
            if let Some(date) = date {
                circumstances.push(format!("on {date}"));
            }
            if let Some(time) = time {
                circumstances.push(format!("at {time}"));
            }
            if let Some(location) = location {
                circumstances.push(format!("in {location}"));
            }
            if let Some(instrument) = instrument {
                circumstances.push(format!("using {instrument}"));
            }
            if let Some(manner) = manner {
                circumstances.push(format!("in the manner {manner}"));
            }
            let tail = if circumstances.is_empty() {
                String::new()
            } else {
                format!(" {}", circumstances.join(" "))
            };
            let cancellation = if predicate.phase == ApprovedEventPhaseIR::Cancelled {
                format!(
                    " but the {} was cancelled",
                    english_event_noun(predicate_sense)
                )
            } else {
                String::new()
            };
            match (information.voice, information.topic) {
                (ApprovedEventVoiceIR::Active, None)
                | (ApprovedEventVoiceIR::Active, Some(ApprovedEventInformationRoleIR::Agent)) => {
                    Some(format!(
                        "{} {} {theme}{tail}{cancellation}.",
                        agent.as_deref()?,
                        english_semantic_event_predicate(
                            predicate_sense,
                            predicate.phase,
                            ApprovedEventVoiceIR::Active,
                        )
                    ))
                }
                (ApprovedEventVoiceIR::Active, Some(ApprovedEventInformationRoleIR::Theme)) => {
                    Some(format!(
                        "As for {theme}, {} {} it{tail}{cancellation}.",
                        agent.as_deref()?,
                        english_semantic_event_predicate(
                            predicate_sense,
                            predicate.phase,
                            ApprovedEventVoiceIR::Active,
                        )
                    ))
                }
                (ApprovedEventVoiceIR::Active, Some(ApprovedEventInformationRoleIR::Patient)) => {
                    let patient = patient.as_deref()?;
                    let (prefix, circumstance) = if matches!(
                        predicate_sense,
                        ApprovedEventPredicateSenseIR::Deliver
                            | ApprovedEventPredicateSenseIR::Give
                    ) {
                        ("To", format!("to {patient}"))
                    } else {
                        ("For", format!("for {patient}"))
                    };
                    let remaining = circumstances
                        .into_iter()
                        .filter(|part| part != &circumstance)
                        .collect::<Vec<_>>()
                        .join(" ");
                    let remaining = if remaining.is_empty() {
                        String::new()
                    } else {
                        format!(" {remaining}")
                    };
                    Some(format!(
                        "{prefix} {patient}, {} {} {theme}{remaining}{cancellation}.",
                        agent.as_deref()?,
                        english_semantic_event_predicate(
                            predicate_sense,
                            predicate.phase,
                            ApprovedEventVoiceIR::Active,
                        )
                    ))
                }
                (ApprovedEventVoiceIR::Active, Some(ApprovedEventInformationRoleIR::Source)) => {
                    let source = source.as_deref()?;
                    let remaining = circumstances
                        .into_iter()
                        .filter(|part| part != &format!("from {source}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let remaining = if remaining.is_empty() {
                        String::new()
                    } else {
                        format!(" {remaining}")
                    };
                    Some(format!(
                        "From {source}, {} {} {theme}{remaining}{cancellation}.",
                        agent.as_deref()?,
                        english_semantic_event_predicate(
                            predicate_sense,
                            predicate.phase,
                            ApprovedEventVoiceIR::Active,
                        )
                    ))
                }
                (
                    ApprovedEventVoiceIR::Active,
                    Some(ApprovedEventInformationRoleIR::Destination),
                ) => {
                    let destination = destination.as_deref()?;
                    let remaining = circumstances
                        .into_iter()
                        .filter(|part| part != &format!("to {destination}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let remaining = if remaining.is_empty() {
                        String::new()
                    } else {
                        format!(" {remaining}")
                    };
                    Some(format!(
                        "To {destination}, {} {} {theme}{remaining}{cancellation}.",
                        agent.as_deref()?,
                        english_semantic_event_predicate(
                            predicate_sense,
                            predicate.phase,
                            ApprovedEventVoiceIR::Active,
                        ),
                    ))
                }
                (ApprovedEventVoiceIR::Passive, topic) => {
                    let by_agent = if information
                        .omitted_roles
                        .contains(&ApprovedRelationTypeIR::Agent)
                    {
                        String::new()
                    } else {
                        agent
                            .as_deref()
                            .map(|agent| format!(" by {agent}"))
                            .unwrap_or_default()
                    };
                    let (front, passive_tail) = match topic {
                        None | Some(ApprovedEventInformationRoleIR::Theme) => (theme.clone(), tail),
                        Some(ApprovedEventInformationRoleIR::Patient) => {
                            let patient = patient.as_deref()?;
                            let (prefix, circumstance) = if matches!(
                                predicate_sense,
                                ApprovedEventPredicateSenseIR::Deliver
                                    | ApprovedEventPredicateSenseIR::Give
                            ) {
                                ("To", format!("to {patient}"))
                            } else {
                                ("For", format!("for {patient}"))
                            };
                            let remaining = circumstances
                                .into_iter()
                                .filter(|part| part != &circumstance)
                                .collect::<Vec<_>>()
                                .join(" ");
                            let remaining = if remaining.is_empty() {
                                String::new()
                            } else {
                                format!(" {remaining}")
                            };
                            (format!("{prefix} {patient}, {theme}"), remaining)
                        }
                        Some(ApprovedEventInformationRoleIR::Source) => {
                            let source = source.as_deref()?;
                            let remaining = circumstances
                                .into_iter()
                                .filter(|part| part != &format!("from {source}"))
                                .collect::<Vec<_>>()
                                .join(" ");
                            let remaining = if remaining.is_empty() {
                                String::new()
                            } else {
                                format!(" {remaining}")
                            };
                            (format!("From {source}, {theme}"), remaining)
                        }
                        Some(ApprovedEventInformationRoleIR::Destination) => {
                            let destination = destination.as_deref()?;
                            let remaining = circumstances
                                .into_iter()
                                .filter(|part| part != &format!("to {destination}"))
                                .collect::<Vec<_>>()
                                .join(" ");
                            let remaining = if remaining.is_empty() {
                                String::new()
                            } else {
                                format!(" {remaining}")
                            };
                            (format!("To {destination}, {theme}"), remaining)
                        }
                        Some(_) => return None,
                    };
                    Some(format!(
                        "{front} {}{by_agent}{passive_tail}{cancellation}.",
                        english_semantic_event_predicate(
                            predicate_sense,
                            predicate.phase,
                            ApprovedEventVoiceIR::Passive,
                        )
                    ))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn with_particle(value: &str, consonant: &'static str, vowel: &'static str) -> String {
    format!("{value}{}", object_particle(value, consonant, vowel).0)
}

fn with_directional_particle(value: &str) -> String {
    format!("{value}{}", directional_particle(value).0)
}

fn korean_manner_phrase(value: &str) -> String {
    let encoded = value.trim();
    // Text values are quoted in ordinary relation clauses to preserve exact
    // boundaries.  Inside a typed event frame the Manner role already
    // supplies that boundary, so expose the approved text as an adverbial
    // instead of producing `“...하면서” 방식으로`.
    let decoded = decode_text_value(encoded);
    let value = decoded.as_deref().unwrap_or(encoded).trim();
    if value.ends_with("방식") || value.ends_with("방법") {
        with_directional_particle(value)
    } else if [
        " 뒤",
        " 후",
        "하면서",
        "하며",
        "도록",
        "하게",
        "히",
        " 채",
        "채로",
        "에",
        "시",
    ]
    .iter()
    .any(|suffix| value.ends_with(suffix))
    {
        value.to_string()
    } else {
        format!("{value} 방식으로")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EnglishEventLexeme {
    base: &'static str,
    present: &'static str,
    progressive: &'static str,
    past: &'static str,
    participle: &'static str,
    noun: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EventLanguageCodec {
    predicate_sense: ApprovedEventPredicateSenseIR,
    korean_nominal: &'static str,
    korean_inflection: Option<KoreanEventInflection>,
    korean_spoken_inflection: Option<KoreanEventInflection>,
    english: EnglishEventLexeme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KoreanEventInflection {
    active: KoreanVerbForms,
    passive: KoreanVerbForms,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KoreanVerbForms {
    future_adnominal: &'static str,
    present_formal: &'static str,
    present_polite: &'static str,
    present_casual: &'static str,
    progressive: &'static str,
    past_formal: &'static str,
    past_polite: &'static str,
    past_casual: &'static str,
}

macro_rules! korean_verb_forms {
    ($future:literal, $formal:literal, $polite:literal, $casual:literal,
     $progressive:literal, $past_formal:literal, $past_polite:literal, $past_casual:literal $(,)?) => {
        KoreanVerbForms {
            future_adnominal: $future,
            present_formal: $formal,
            present_polite: $polite,
            present_casual: $casual,
            progressive: $progressive,
            past_formal: $past_formal,
            past_polite: $past_polite,
            past_casual: $past_casual,
        }
    };
}

const MOVE_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "옮길",
        "옮깁니다",
        "옮겨요",
        "옮겨",
        "옮기고",
        "옮겼습니다",
        "옮겼어요",
        "옮겼어",
    ),
    passive: korean_verb_forms!(
        "옮겨질",
        "옮겨집니다",
        "옮겨져요",
        "옮겨져",
        "옮겨지고",
        "옮겨졌습니다",
        "옮겨졌어요",
        "옮겨졌어",
    ),
};

const SEND_SPOKEN_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "보낼",
        "보냅니다",
        "보내요",
        "보내",
        "보내고",
        "보냈습니다",
        "보냈어요",
        "보냈어",
    ),
    passive: korean_verb_forms!(
        "보내질",
        "보내집니다",
        "보내져요",
        "보내져",
        "보내지고",
        "보내졌습니다",
        "보내졌어요",
        "보내졌어",
    ),
};

const DELIVER_SPOKEN_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "전해 줄",
        "전해 줍니다",
        "전해 줘요",
        "전해 줘",
        "전해 주고",
        "전해 줬습니다",
        "전해 줬어요",
        "전해 줬어",
    ),
    passive: korean_verb_forms!(
        "전해질",
        "전해집니다",
        "전해져요",
        "전해져",
        "전해지고",
        "전해졌습니다",
        "전해졌어요",
        "전해졌어",
    ),
};

const GIVE_SPOKEN_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "줄",
        "줍니다",
        "줘요",
        "줘",
        "주고",
        "줬습니다",
        "줬어요",
        "줬어",
    ),
    passive: korean_verb_forms!(
        "주어질",
        "주어집니다",
        "주어져요",
        "주어져",
        "주어지고",
        "주어졌습니다",
        "주어졌어요",
        "주어졌어",
    ),
};

const APPLY_FORMAL_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "도포할",
        "도포합니다",
        "도포해요",
        "도포해",
        "도포하고",
        "도포했습니다",
        "도포했어요",
        "도포했어",
    ),
    passive: korean_verb_forms!(
        "도포될",
        "도포됩니다",
        "도포돼요",
        "도포돼",
        "도포되고",
        "도포됐습니다",
        "도포됐어요",
        "도포됐어",
    ),
};

const APPLY_SPOKEN_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "바를",
        "바릅니다",
        "발라요",
        "발라",
        "바르고",
        "발랐습니다",
        "발랐어요",
        "발랐어",
    ),
    passive: korean_verb_forms!(
        "발라질",
        "발라집니다",
        "발라져요",
        "발라져",
        "발라지고",
        "발라졌습니다",
        "발라졌어요",
        "발라졌어",
    ),
};

const ATTACH_FORMAL_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "부착할",
        "부착합니다",
        "부착해요",
        "부착해",
        "부착하고",
        "부착했습니다",
        "부착했어요",
        "부착했어",
    ),
    passive: korean_verb_forms!(
        "부착될",
        "부착됩니다",
        "부착돼요",
        "부착돼",
        "부착되고",
        "부착됐습니다",
        "부착됐어요",
        "부착됐어",
    ),
};

const ATTACH_SPOKEN_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "붙일",
        "붙입니다",
        "붙여요",
        "붙여",
        "붙이고",
        "붙였습니다",
        "붙였어요",
        "붙였어",
    ),
    passive: korean_verb_forms!(
        "붙을",
        "붙습니다",
        "붙어요",
        "붙어",
        "붙고",
        "붙었습니다",
        "붙었어요",
        "붙었어",
    ),
};

const CREATE_SPOKEN_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "만들",
        "만듭니다",
        "만들어요",
        "만들어",
        "만들고",
        "만들었습니다",
        "만들었어요",
        "만들었어",
    ),
    passive: korean_verb_forms!(
        "만들어질",
        "만들어집니다",
        "만들어져요",
        "만들어져",
        "만들어지고",
        "만들어졌습니다",
        "만들어졌어요",
        "만들어졌어",
    ),
};

const INSPECT_FORMAL_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "점검할",
        "점검합니다",
        "점검해요",
        "점검해",
        "점검하고",
        "점검했습니다",
        "점검했어요",
        "점검했어",
    ),
    passive: korean_verb_forms!(
        "점검될",
        "점검됩니다",
        "점검돼요",
        "점검돼",
        "점검되고",
        "점검됐습니다",
        "점검됐어요",
        "점검됐어",
    ),
};

const START_FORMAL_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "시작할",
        "시작합니다",
        "시작해요",
        "시작해",
        "시작하고",
        "시작했습니다",
        "시작했어요",
        "시작했어",
    ),
    passive: korean_verb_forms!(
        "시작될",
        "시작됩니다",
        "시작돼요",
        "시작돼",
        "시작되고",
        "시작됐습니다",
        "시작됐어요",
        "시작됐어",
    ),
};

const CHANGE_SPOKEN_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "바꿀",
        "바꿉니다",
        "바꿔요",
        "바꿔",
        "바꾸고",
        "바꿨습니다",
        "바꿨어요",
        "바꿨어",
    ),
    passive: korean_verb_forms!(
        "바뀔",
        "바뀝니다",
        "바뀌어요",
        "바뀌어",
        "바뀌고",
        "바뀌었습니다",
        "바뀌었어요",
        "바뀌었어",
    ),
};

const CONVERT_FORMAL_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "전환할",
        "전환합니다",
        "전환해요",
        "전환해",
        "전환하고",
        "전환했습니다",
        "전환했어요",
        "전환했어",
    ),
    passive: korean_verb_forms!(
        "전환될",
        "전환됩니다",
        "전환돼요",
        "전환돼",
        "전환되고",
        "전환됐습니다",
        "전환됐어요",
        "전환됐어",
    ),
};

const INSTALL_FORMAL_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "설치할",
        "설치합니다",
        "설치해요",
        "설치해",
        "설치하고",
        "설치했습니다",
        "설치했어요",
        "설치했어",
    ),
    passive: korean_verb_forms!(
        "설치될",
        "설치됩니다",
        "설치돼요",
        "설치돼",
        "설치되고",
        "설치됐습니다",
        "설치됐어요",
        "설치됐어",
    ),
};

const BUILD_FORMAL_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "시공할",
        "시공합니다",
        "시공해요",
        "시공해",
        "시공하고",
        "시공했습니다",
        "시공했어요",
        "시공했어",
    ),
    passive: korean_verb_forms!(
        "시공될",
        "시공됩니다",
        "시공돼요",
        "시공돼",
        "시공되고",
        "시공됐습니다",
        "시공됐어요",
        "시공됐어",
    ),
};

const OPERATE_FORMAL_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "운영할",
        "운영합니다",
        "운영해요",
        "운영해",
        "운영하고",
        "운영했습니다",
        "운영했어요",
        "운영했어",
    ),
    passive: korean_verb_forms!(
        "운영될",
        "운영됩니다",
        "운영돼요",
        "운영돼",
        "운영되고",
        "운영됐습니다",
        "운영됐어요",
        "운영됐어",
    ),
};

const COMPLETE_SPOKEN_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "마칠",
        "마칩니다",
        "마쳐요",
        "마쳐",
        "마치고",
        "마쳤습니다",
        "마쳤어요",
        "마쳤어",
    ),
    passive: korean_verb_forms!(
        "끝날",
        "끝납니다",
        "끝나요",
        "끝나",
        "끝나고",
        "끝났습니다",
        "끝났어요",
        "끝났어",
    ),
};

const OPEN_SPOKEN_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "열",
        "엽니다",
        "열어요",
        "열어",
        "열고",
        "열었습니다",
        "열었어요",
        "열었어",
    ),
    passive: korean_verb_forms!(
        "열릴",
        "열립니다",
        "열려요",
        "열려",
        "열리고",
        "열렸습니다",
        "열렸어요",
        "열렸어",
    ),
};

const CLOSE_SPOKEN_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "닫을",
        "닫습니다",
        "닫아요",
        "닫아",
        "닫고",
        "닫았습니다",
        "닫았어요",
        "닫았어",
    ),
    passive: korean_verb_forms!(
        "닫힐",
        "닫힙니다",
        "닫혀요",
        "닫혀",
        "닫히고",
        "닫혔습니다",
        "닫혔어요",
        "닫혔어",
    ),
};

const FILL_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "메울",
        "메웁니다",
        "메워요",
        "메워",
        "메우고",
        "메웠습니다",
        "메웠어요",
        "메웠어",
    ),
    passive: korean_verb_forms!(
        "메워질",
        "메워집니다",
        "메워져요",
        "메워져",
        "메워지고",
        "메워졌습니다",
        "메워졌어요",
        "메워졌어",
    ),
};

const WITHDRAW_KOREAN: KoreanEventInflection = KoreanEventInflection {
    active: korean_verb_forms!(
        "후퇴할",
        "후퇴합니다",
        "후퇴해요",
        "후퇴해",
        "후퇴하고",
        "후퇴했습니다",
        "후퇴했어요",
        "후퇴했어",
    ),
    // Withdraw is realized through the moving Theme even for a passive event
    // perspective, so both voice entries intentionally use the same Korean
    // intransitive forms.
    passive: korean_verb_forms!(
        "후퇴할",
        "후퇴합니다",
        "후퇴해요",
        "후퇴해",
        "후퇴하고",
        "후퇴했습니다",
        "후퇴했어요",
        "후퇴했어",
    ),
};

const EVENT_LANGUAGE_CODECS: &[EventLanguageCodec] = &[
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Move,
        korean_nominal: "이동",
        korean_inflection: Some(MOVE_KOREAN),
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "move",
            present: "moves",
            progressive: "moving",
            past: "moved",
            participle: "moved",
            noun: "movement",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Transfer,
        korean_nominal: "전송",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "transfer",
            present: "transfers",
            progressive: "transferring",
            past: "transferred",
            participle: "transferred",
            noun: "transfer",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Deliver,
        korean_nominal: "전달",
        korean_inflection: None,
        korean_spoken_inflection: Some(DELIVER_SPOKEN_KOREAN),
        english: EnglishEventLexeme {
            base: "deliver",
            present: "delivers",
            progressive: "delivering",
            past: "delivered",
            participle: "delivered",
            noun: "delivery",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Send,
        korean_nominal: "발송",
        korean_inflection: None,
        korean_spoken_inflection: Some(SEND_SPOKEN_KOREAN),
        english: EnglishEventLexeme {
            base: "send",
            present: "sends",
            progressive: "sending",
            past: "sent",
            participle: "sent",
            noun: "shipment",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Share,
        korean_nominal: "공유",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "share",
            present: "shares",
            progressive: "sharing",
            past: "shared",
            participle: "shared",
            noun: "sharing",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Give,
        korean_nominal: "제공",
        korean_inflection: None,
        korean_spoken_inflection: Some(GIVE_SPOKEN_KOREAN),
        english: EnglishEventLexeme {
            base: "give",
            present: "gives",
            progressive: "giving",
            past: "gave",
            participle: "given",
            noun: "provision",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Apply,
        korean_nominal: "도포",
        korean_inflection: Some(APPLY_FORMAL_KOREAN),
        korean_spoken_inflection: Some(APPLY_SPOKEN_KOREAN),
        english: EnglishEventLexeme {
            base: "apply",
            present: "applies",
            progressive: "applying",
            past: "applied",
            participle: "applied",
            noun: "application",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Attach,
        korean_nominal: "부착",
        korean_inflection: Some(ATTACH_FORMAL_KOREAN),
        korean_spoken_inflection: Some(ATTACH_SPOKEN_KOREAN),
        english: EnglishEventLexeme {
            base: "attach",
            present: "attaches",
            progressive: "attaching",
            past: "attached",
            participle: "attached",
            noun: "attachment",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Upload,
        korean_nominal: "업로드",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "upload",
            present: "uploads",
            progressive: "uploading",
            past: "uploaded",
            participle: "uploaded",
            noun: "upload",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Create,
        korean_nominal: "생성",
        korean_inflection: None,
        korean_spoken_inflection: Some(CREATE_SPOKEN_KOREAN),
        english: EnglishEventLexeme {
            base: "create",
            present: "creates",
            progressive: "creating",
            past: "created",
            participle: "created",
            noun: "creation",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Write,
        korean_nominal: "작성",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "write",
            present: "writes",
            progressive: "writing",
            past: "wrote",
            participle: "written",
            noun: "writing",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Inspect,
        korean_nominal: "점검",
        korean_inflection: Some(INSPECT_FORMAL_KOREAN),
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "inspect",
            present: "inspects",
            progressive: "inspecting",
            past: "inspected",
            participle: "inspected",
            noun: "inspection",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Change,
        korean_nominal: "변경",
        korean_inflection: None,
        korean_spoken_inflection: Some(CHANGE_SPOKEN_KOREAN),
        english: EnglishEventLexeme {
            base: "change",
            present: "changes",
            progressive: "changing",
            past: "changed",
            participle: "changed",
            noun: "change",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Record,
        korean_nominal: "기록",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "record",
            present: "records",
            progressive: "recording",
            past: "recorded",
            participle: "recorded",
            noun: "record",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Convert,
        korean_nominal: "전환",
        korean_inflection: Some(CONVERT_FORMAL_KOREAN),
        korean_spoken_inflection: Some(CONVERT_FORMAL_KOREAN),
        english: EnglishEventLexeme {
            base: "convert",
            present: "converts",
            progressive: "converting",
            past: "converted",
            participle: "converted",
            noun: "conversion",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Install,
        korean_nominal: "설치",
        korean_inflection: Some(INSTALL_FORMAL_KOREAN),
        korean_spoken_inflection: Some(INSTALL_FORMAL_KOREAN),
        english: EnglishEventLexeme {
            base: "install",
            present: "installs",
            progressive: "installing",
            past: "installed",
            participle: "installed",
            noun: "installation",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Build,
        korean_nominal: "시공",
        korean_inflection: Some(BUILD_FORMAL_KOREAN),
        korean_spoken_inflection: Some(BUILD_FORMAL_KOREAN),
        english: EnglishEventLexeme {
            base: "build",
            present: "builds",
            progressive: "building",
            past: "built",
            participle: "built",
            noun: "construction",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Operate,
        korean_nominal: "운영",
        korean_inflection: Some(OPERATE_FORMAL_KOREAN),
        korean_spoken_inflection: Some(OPERATE_FORMAL_KOREAN),
        english: EnglishEventLexeme {
            base: "operate",
            present: "operates",
            progressive: "operating",
            past: "operated",
            participle: "operated",
            noun: "operation",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Post,
        korean_nominal: "게시",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "post",
            present: "posts",
            progressive: "posting",
            past: "posted",
            participle: "posted",
            noun: "posting",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Supply,
        korean_nominal: "공급",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "supply",
            present: "supplies",
            progressive: "supplying",
            past: "supplied",
            participle: "supplied",
            noun: "supply",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Activate,
        korean_nominal: "활성화",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "activate",
            present: "activates",
            progressive: "activating",
            past: "activated",
            participle: "activated",
            noun: "activation",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Deactivate,
        korean_nominal: "비활성화",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "deactivate",
            present: "deactivates",
            progressive: "deactivating",
            past: "deactivated",
            participle: "deactivated",
            noun: "deactivation",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Reset,
        korean_nominal: "재설정",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "reset",
            present: "resets",
            progressive: "resetting",
            past: "reset",
            participle: "reset",
            noun: "reset",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Expand,
        korean_nominal: "확대",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "expand",
            present: "expands",
            progressive: "expanding",
            past: "expanded",
            participle: "expanded",
            noun: "expansion",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Start,
        korean_nominal: "시작",
        korean_inflection: Some(START_FORMAL_KOREAN),
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "start",
            present: "starts",
            progressive: "starting",
            past: "started",
            participle: "started",
            noun: "start",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Reduce,
        korean_nominal: "축소",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "reduce",
            present: "reduces",
            progressive: "reducing",
            past: "reduced",
            participle: "reduced",
            noun: "reduction",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Remove,
        korean_nominal: "제거",
        korean_inflection: None,
        korean_spoken_inflection: None,
        english: EnglishEventLexeme {
            base: "remove",
            present: "removes",
            progressive: "removing",
            past: "removed",
            participle: "removed",
            noun: "removal",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Complete,
        korean_nominal: "완료",
        korean_inflection: None,
        korean_spoken_inflection: Some(COMPLETE_SPOKEN_KOREAN),
        english: EnglishEventLexeme {
            base: "complete",
            present: "completes",
            progressive: "completing",
            past: "completed",
            participle: "completed",
            noun: "completion",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Open,
        korean_nominal: "개방",
        korean_inflection: None,
        korean_spoken_inflection: Some(OPEN_SPOKEN_KOREAN),
        english: EnglishEventLexeme {
            base: "open",
            present: "opens",
            progressive: "opening",
            past: "opened",
            participle: "opened",
            noun: "opening",
        },
    },
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Close,
        korean_nominal: "폐쇄",
        korean_inflection: None,
        korean_spoken_inflection: Some(CLOSE_SPOKEN_KOREAN),
        english: EnglishEventLexeme {
            base: "close",
            present: "closes",
            progressive: "closing",
            past: "closed",
            participle: "closed",
            noun: "closure",
        },
    },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Withdraw, korean_nominal: "후퇴", korean_inflection: Some(WITHDRAW_KOREAN), korean_spoken_inflection: Some(WITHDRAW_KOREAN), english: EnglishEventLexeme { base: "withdraw", present: "withdraws", progressive: "withdrawing", past: "withdrew", participle: "withdrawn", noun: "withdrawal" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Heat, korean_nominal: "가열", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "heat", present: "heats", progressive: "heating", past: "heated", participle: "heated", noun: "heating" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Align, korean_nominal: "정렬", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "align", present: "aligns", progressive: "aligning", past: "aligned", participle: "aligned", noun: "alignment" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Calibrate, korean_nominal: "보정", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "calibrate", present: "calibrates", progressive: "calibrating", past: "calibrated", participle: "calibrated", noun: "calibration" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Seal, korean_nominal: "밀폐", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "seal", present: "seals", progressive: "sealing", past: "sealed", participle: "sealed", noun: "sealing" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Press, korean_nominal: "압착", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "press", present: "presses", progressive: "pressing", past: "pressed", participle: "pressed", noun: "pressing" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Replace, korean_nominal: "교체", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "replace", present: "replaces", progressive: "replacing", past: "replaced", participle: "replaced", noun: "replacement" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Dry, korean_nominal: "건조", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "dry", present: "dries", progressive: "drying", past: "dried", participle: "dried", noun: "drying" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Clean, korean_nominal: "세척", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "clean", present: "cleans", progressive: "cleaning", past: "cleaned", participle: "cleaned", noun: "cleaning" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Reinforce, korean_nominal: "보강", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "reinforce", present: "reinforces", progressive: "reinforcing", past: "reinforced", participle: "reinforced", noun: "reinforcement" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Connect, korean_nominal: "연계", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "connect", present: "connects", progressive: "connecting", past: "connected", participle: "connected", noun: "connection" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Compare, korean_nominal: "대조", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "compare", present: "compares", progressive: "comparing", past: "compared", participle: "compared", noun: "comparison" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Fill, korean_nominal: "충전", korean_inflection: Some(FILL_KOREAN), korean_spoken_inflection: Some(FILL_KOREAN), english: EnglishEventLexeme { base: "fill", present: "fills", progressive: "filling", past: "filled", participle: "filled", noun: "filling" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Repair, korean_nominal: "보수", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "repair", present: "repairs", progressive: "repairing", past: "repaired", participle: "repaired", noun: "repair" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Observe, korean_nominal: "관측", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "observe", present: "observes", progressive: "observing", past: "observed", participle: "observed", noun: "observation" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Deploy, korean_nominal: "투입", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "deploy", present: "deploys", progressive: "deploying", past: "deployed", participle: "deployed", noun: "deployment" } },
    EventLanguageCodec { predicate_sense: ApprovedEventPredicateSenseIR::Reuse, korean_nominal: "재사용", korean_inflection: None, korean_spoken_inflection: None, english: EnglishEventLexeme { base: "reuse", present: "reuses", progressive: "reusing", past: "reused", participle: "reused", noun: "reuse" } },
];

fn event_language_codec(predicate_sense: ApprovedEventPredicateSenseIR) -> EventLanguageCodec {
    EVENT_LANGUAGE_CODECS
        .iter()
        .copied()
        .find(|codec| codec.predicate_sense == predicate_sense)
        .expect("each approved predicate sense has a language codec")
}

fn korean_semantic_event_predicate(
    predicate_sense: ApprovedEventPredicateSenseIR,
    phase: ApprovedEventPhaseIR,
    voice: ApprovedEventVoiceIR,
    register: LanguageRegisterIR,
) -> String {
    let codec = event_language_codec(predicate_sense);
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let inflection = if formal {
        codec.korean_inflection
    } else {
        codec.korean_spoken_inflection.or(codec.korean_inflection)
    };
    if let Some(inflection) = inflection {
        return {
            let forms = match voice {
                ApprovedEventVoiceIR::Active => inflection.active,
                ApprovedEventVoiceIR::Passive => inflection.passive,
            };
            match (phase, formal, casual) {
                (ApprovedEventPhaseIR::Planned, true, _) => {
                    format!("{} 예정입니다", forms.future_adnominal)
                }
                (ApprovedEventPhaseIR::Planned, false, false) => {
                    format!("{} 거예요", forms.future_adnominal)
                }
                (ApprovedEventPhaseIR::Planned, false, true) => {
                    format!("{} 거야", forms.future_adnominal)
                }
                (ApprovedEventPhaseIR::Scheduled, true, _) => forms.present_formal.into(),
                (ApprovedEventPhaseIR::Scheduled, false, false) => forms.present_polite.into(),
                (ApprovedEventPhaseIR::Scheduled, false, true) => forms.present_casual.into(),
                (ApprovedEventPhaseIR::Ongoing, true, _) => {
                    format!("{} 있습니다", forms.progressive)
                }
                (ApprovedEventPhaseIR::Ongoing, false, false) => {
                    format!("{} 있어요", forms.progressive)
                }
                (ApprovedEventPhaseIR::Ongoing, false, true) => {
                    format!("{} 있어", forms.progressive)
                }
                (ApprovedEventPhaseIR::Completed, true, _) => forms.past_formal.into(),
                (ApprovedEventPhaseIR::Completed, false, false) => forms.past_polite.into(),
                (ApprovedEventPhaseIR::Completed, false, true) => forms.past_casual.into(),
                (ApprovedEventPhaseIR::Cancelled, true, _) => {
                    format!("{} 예정이었지만 취소됐습니다", forms.future_adnominal)
                }
                (ApprovedEventPhaseIR::Cancelled, false, false) => {
                    format!("{} 거였는데 취소됐어요", forms.future_adnominal)
                }
                (ApprovedEventPhaseIR::Cancelled, false, true) => {
                    format!("{} 거였는데 취소됐어", forms.future_adnominal)
                }
            }
        };
    }

    let nominal = codec.korean_nominal;
    let suffix = match (voice, phase, formal, casual) {
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Planned, true, _) => "할 예정입니다",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Planned, false, false) => "할 거예요",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Planned, false, true) => "할 거야",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Scheduled, true, _) => "합니다",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Scheduled, false, false) => "해요",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Scheduled, false, true) => "해",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Ongoing, true, _) => " 중입니다",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Ongoing, false, false) => " 중이에요",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Ongoing, false, true) => " 중이야",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Completed, true, _) => "했습니다",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Completed, false, false) => "했어요",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Completed, false, true) => "했어",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Cancelled, true, _) => {
            "할 예정이었지만 취소됐습니다"
        }
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Cancelled, false, false) => {
            "하려고 했는데 취소됐어요"
        }
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Cancelled, false, true) => {
            "하려고 했는데 취소됐어"
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Planned, true, _) => "될 예정입니다",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Planned, false, false) => "될 거예요",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Planned, false, true) => "될 거야",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Scheduled, true, _) => "됩니다",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Scheduled, false, false) => "돼요",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Scheduled, false, true) => "돼",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Ongoing, true, _) => "되고 있습니다",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Ongoing, false, false) => {
            "되고 있어요"
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Ongoing, false, true) => "되고 있어",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Completed, true, _) => "됐습니다",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Completed, false, false) => "됐어요",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Completed, false, true) => "됐어",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Cancelled, true, _) => {
            "될 예정이었지만 취소됐습니다"
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Cancelled, false, false) => {
            "되려고 했는데 취소됐어요"
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Cancelled, false, true) => {
            "되려고 했는데 취소됐어"
        }
    };
    format!("{nominal}{suffix}")
}

fn english_event_lexeme(predicate_sense: ApprovedEventPredicateSenseIR) -> EnglishEventLexeme {
    event_language_codec(predicate_sense).english
}

fn english_event_noun(predicate_sense: ApprovedEventPredicateSenseIR) -> &'static str {
    english_event_lexeme(predicate_sense).noun
}

fn english_semantic_event_predicate(
    predicate_sense: ApprovedEventPredicateSenseIR,
    phase: ApprovedEventPhaseIR,
    voice: ApprovedEventVoiceIR,
) -> String {
    let lexeme = english_event_lexeme(predicate_sense);
    match (voice, phase) {
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Planned) => {
            format!("will {}", lexeme.base)
        }
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Scheduled) => lexeme.present.into(),
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Ongoing) => {
            format!("is {}", lexeme.progressive)
        }
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Completed) => lexeme.past.into(),
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Cancelled) => {
            format!("was scheduled to {}", lexeme.base)
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Planned) => {
            format!("will be {}", lexeme.participle)
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Scheduled) => {
            format!("is {}", lexeme.participle)
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Ongoing) => {
            format!("is being {}", lexeme.participle)
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Completed) => {
            format!("was {}", lexeme.participle)
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Cancelled) => {
            format!("was scheduled to be {}", lexeme.participle)
        }
    }
}

fn korean_event_predicate(
    kind: DocumentEventPredicateKindIR,
    phase: ApprovedEventPhaseIR,
    register: LanguageRegisterIR,
) -> Option<&'static str> {
    let pair = match (kind, phase) {
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Planned) => {
            ("열릴 예정입니다", "열릴 예정이에요")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Scheduled) => {
            ("열립니다", "열려요")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Ongoing) => {
            ("열리고 있습니다", "열리고 있어요")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Completed) => {
            ("끝났습니다", "끝났어요")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Cancelled) => (
            "열릴 예정이었지만 취소됐습니다",
            "열릴 예정이었지만 취소됐어요",
        ),
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Planned) => {
            ("진행될 예정입니다", "진행될 예정이에요")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Scheduled) => {
            ("진행됩니다", "진행돼요")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Ongoing) => {
            ("진행 중입니다", "진행 중이에요")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Completed) => {
            ("완료됐습니다", "완료됐어요")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Cancelled) => (
            "진행될 예정이었지만 취소됐습니다",
            "진행될 예정이었지만 취소됐어요",
        ),
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Planned) => {
            ("출발할 예정입니다", "출발할 예정이에요")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Scheduled) => {
            ("출발합니다", "출발해요")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Completed) => {
            ("출발했습니다", "출발했어요")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Cancelled) => (
            "출발할 예정이었지만 취소됐습니다",
            "출발할 예정이었지만 취소됐어요",
        ),
        (DocumentEventPredicateKindIR::Arrival, ApprovedEventPhaseIR::Planned) => {
            ("도착할 예정입니다", "도착할 예정이에요")
        }
        (DocumentEventPredicateKindIR::Arrival, ApprovedEventPhaseIR::Scheduled) => {
            ("도착합니다", "도착해요")
        }
        (DocumentEventPredicateKindIR::Arrival, ApprovedEventPhaseIR::Completed) => {
            ("도착했습니다", "도착했어요")
        }
        _ => return None,
    };
    if register == LanguageRegisterIR::Formal {
        return Some(pair.0);
    }
    if register == LanguageRegisterIR::Neutral {
        return Some(pair.1);
    }
    match (kind, phase) {
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Planned) => {
            Some("열릴 예정이야")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Scheduled) => {
            Some("열려")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Ongoing) => {
            Some("열리고 있어")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Completed) => {
            Some("끝났어")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Cancelled) => {
            Some("열릴 예정이었지만 취소됐어")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Planned) => {
            Some("진행될 예정이야")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Scheduled) => {
            Some("진행돼")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Ongoing) => {
            Some("진행 중이야")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Completed) => {
            Some("완료됐어")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Cancelled) => {
            Some("진행될 예정이었지만 취소됐어")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Planned) => {
            Some("출발할 예정이야")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Scheduled) => {
            Some("출발해")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Completed) => {
            Some("출발했어")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Cancelled) => {
            Some("출발할 예정이었지만 취소됐어")
        }
        (DocumentEventPredicateKindIR::Arrival, ApprovedEventPhaseIR::Planned) => {
            Some("도착할 예정이야")
        }
        (DocumentEventPredicateKindIR::Arrival, ApprovedEventPhaseIR::Scheduled) => Some("도착해"),
        (DocumentEventPredicateKindIR::Arrival, ApprovedEventPhaseIR::Completed) => {
            Some("도착했어")
        }
        _ => None,
    }
}

fn english_event_predicate(
    kind: DocumentEventPredicateKindIR,
    phase: ApprovedEventPhaseIR,
) -> Option<&'static str> {
    match (kind, phase) {
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Planned) => {
            Some("is planned to take place")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Scheduled) => {
            Some("takes place")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Ongoing) => {
            Some("is taking place")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Completed) => {
            Some("has ended")
        }
        (DocumentEventPredicateKindIR::HostedOccurrence, ApprovedEventPhaseIR::Cancelled) => {
            Some("was planned to take place but has been cancelled")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Planned) => {
            Some("is planned to be carried out")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Scheduled) => {
            Some("is carried out")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Ongoing) => {
            Some("is in progress")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Completed) => {
            Some("has been completed")
        }
        (DocumentEventPredicateKindIR::ScheduledProcess, ApprovedEventPhaseIR::Cancelled) => {
            Some("was planned to be carried out but has been cancelled")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Planned) => {
            Some("is planned to depart")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Scheduled) => {
            Some("departs")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Completed) => {
            Some("departed")
        }
        (DocumentEventPredicateKindIR::Departure, ApprovedEventPhaseIR::Cancelled) => {
            Some("was planned to depart but has been cancelled")
        }
        (DocumentEventPredicateKindIR::Arrival, ApprovedEventPhaseIR::Planned) => {
            Some("is planned to arrive")
        }
        (DocumentEventPredicateKindIR::Arrival, ApprovedEventPhaseIR::Scheduled) => Some("arrives"),
        (DocumentEventPredicateKindIR::Arrival, ApprovedEventPhaseIR::Completed) => Some("arrived"),
        _ => None,
    }
}

fn fuse_clause_surfaces(left: &str, right: &str, language: LanguageCodeIR) -> Option<String> {
    match language {
        LanguageCodeIR::Korean => {
            let left = korean_coordinated_surface(left)?;
            let right = right.strip_prefix("또한, ").unwrap_or(right);
            Some(format!("{left} {right}"))
        }
        LanguageCodeIR::English => {
            let left = left.strip_suffix('.')?;
            let right = right.strip_prefix("Also, ").unwrap_or(right);
            Some(format!("{left}, and {right}"))
        }
        _ => None,
    }
}

fn korean_coordinated_surface(surface: &str) -> Option<String> {
    let sentence = surface.strip_suffix('.')?;
    const ENDINGS: &[(&str, &str)] = &[
        ("되지 않았습니다", "되지 않았고,"),
        ("되지 않았어요", "되지 않았고,"),
        ("되지 않았어", "되지 않았고,"),
        ("할 수 없습니다", "할 수 없고,"),
        ("할 수 없어요", "할 수 없고,"),
        ("할 수 없어", "할 수 없고,"),
        ("할 수 있습니다", "할 수 있고,"),
        ("할 수 있어요", "할 수 있고,"),
        ("할 수 있어", "할 수 있고,"),
        ("변경됐습니다", "변경됐고,"),
        ("변경됐어요", "변경됐고,"),
        ("바뀌었어요", "바뀌었고,"),
        ("바뀌었어", "바뀌었고,"),
        ("아닙니다", "아니고,"),
        ("아니에요", "아니고,"),
        ("아니야", "아니고,"),
        ("됐습니다", "됐고,"),
        ("됐어요", "됐고,"),
        ("됐어", "됐고,"),
        ("입니다", "이고,"),
    ];
    for &(finite, coordinated) in ENDINGS {
        if let Some(stem) = sentence.strip_suffix(finite) {
            return Some(format!("{stem}{coordinated}"));
        }
    }
    for (finite, coordinated) in [
        ("이에요", "이고,"),
        ("예요", "이고,"),
        ("이야", "이고,"),
        ("야", "이고,"),
        ("맞습니다", "맞고,"),
        ("맞아요", "맞고,"),
        ("맞아", "맞고,"),
    ] {
        if let Some(stem) = sentence.strip_suffix(finite) {
            return Some(format!("{stem}{coordinated}"));
        }
    }
    None
}

fn rhetorical_prefix(
    rhetorical_move: DocumentRhetoricalMoveIR,
    relation: ApprovedDiscourseRelationIR,
    first_claim: bool,
    language: LanguageCodeIR,
) -> Option<&'static str> {
    match (language, rhetorical_move, relation, first_claim) {
        (
            LanguageCodeIR::Korean,
            DocumentRhetoricalMoveIR::Correction,
            ApprovedDiscourseRelationIR::Correction,
            true,
        ) => Some("정정된 내용은 다음과 같습니다. "),
        (
            LanguageCodeIR::English,
            DocumentRhetoricalMoveIR::Correction,
            ApprovedDiscourseRelationIR::Correction,
            true,
        ) => Some("Correction: "),
        (
            LanguageCodeIR::Korean,
            DocumentRhetoricalMoveIR::Conclusion,
            ApprovedDiscourseRelationIR::Cause,
            _,
        ) => Some("따라서, "),
        (
            LanguageCodeIR::English,
            DocumentRhetoricalMoveIR::Conclusion,
            ApprovedDiscourseRelationIR::Cause,
            _,
        ) => Some("Therefore, "),
        (
            LanguageCodeIR::Korean,
            DocumentRhetoricalMoveIR::Conclusion,
            ApprovedDiscourseRelationIR::Explanation,
            _,
        ) => Some("정리하면, "),
        (
            LanguageCodeIR::English,
            DocumentRhetoricalMoveIR::Conclusion,
            ApprovedDiscourseRelationIR::Explanation,
            _,
        ) => Some("In summary, "),
        (
            LanguageCodeIR::Korean,
            DocumentRhetoricalMoveIR::Conclusion,
            ApprovedDiscourseRelationIR::Condition,
            _,
        ) => Some("이 조건이 충족되면, "),
        (
            LanguageCodeIR::English,
            DocumentRhetoricalMoveIR::Conclusion,
            ApprovedDiscourseRelationIR::Condition,
            _,
        ) => Some("If that condition is met, "),
        (
            LanguageCodeIR::Korean,
            DocumentRhetoricalMoveIR::Concurrent,
            ApprovedDiscourseRelationIR::Simultaneous,
            false,
        ) => Some("동시에, "),
        (
            LanguageCodeIR::English,
            DocumentRhetoricalMoveIR::Concurrent,
            ApprovedDiscourseRelationIR::Simultaneous,
            false,
        ) => Some("At the same time, "),
        _ => None,
    }
}

fn apply_rhetorical_prefix(
    surface: &str,
    rhetorical_prefix: &str,
    language: LanguageCodeIR,
) -> String {
    let sentence = surface.strip_suffix('.').unwrap_or(surface);
    let sentence = match language {
        LanguageCodeIR::Korean => sentence
            .strip_prefix("또한, ")
            .unwrap_or(sentence)
            .to_string(),
        LanguageCodeIR::English => {
            let sentence = sentence.strip_prefix("Also, ").unwrap_or(sentence);
            if rhetorical_prefix.ends_with(", ") {
                sentence
                    .strip_prefix("The ")
                    .map(|rest| format!("the {rest}"))
                    .unwrap_or_else(|| sentence.to_string())
            } else {
                sentence.to_string()
            }
        }
        _ => sentence.to_string(),
    };
    format!("{rhetorical_prefix}{sentence}.")
}

fn realize_followup_claim(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> String {
    let full = realize_claim(claim, response, language);
    let full = if response.speech_act == ApprovedSpeechActIR::Reassure
        && language == LanguageCodeIR::Korean
    {
        let prefix = format!("{} ", korean_reassurance_marker(response.style.register));
        full.strip_prefix(&prefix).unwrap_or(&full).to_string()
    } else {
        full
    };
    let (sentence, punctuation) = full
        .char_indices()
        .next_back()
        .filter(|(_, character)| matches!(character, '.' | '?' | '!' | '。' | '？' | '！'))
        .map(|(index, character)| (&full[..index], character))
        .unwrap_or((full.as_str(), '.'));
    match language {
        LanguageCodeIR::Korean => {
            let topic = object_particle(&claim.subject.canonical_lexical_label, "은", "는").0;
            let topic_prefix = format!("{}{topic} ", claim.subject.canonical_lexical_label);
            let plain_prefix = format!("{} ", claim.subject.canonical_lexical_label);
            sentence
                .strip_prefix(&topic_prefix)
                .or_else(|| sentence.strip_prefix(&plain_prefix))
                .map(|followup| format!("또한, {followup}{punctuation}"))
                .unwrap_or(full)
        }
        LanguageCodeIR::English => {
            let subject_prefix = format!("{} ", claim.subject.canonical_lexical_label);
            if let Some(predicate) = sentence.strip_prefix(&subject_prefix) {
                return format!("Also, it {predicate}{punctuation}");
            }
            let relation = relation_label(claim.relation, LanguageCodeIR::English);
            let relational_prefix = format!(
                "The {relation} of {} ",
                claim.subject.canonical_lexical_label
            );
            sentence
                .strip_prefix(&relational_prefix)
                .map(|predicate| format!("Also, its {relation} {predicate}{punctuation}"))
                .unwrap_or(full)
        }
        _ => full,
    }
}

fn numeric_value(value: &ApprovedOpenValueIR) -> Option<i64> {
    match value {
        ApprovedOpenValueIR::Integer(value) => Some(*value),
        ApprovedOpenValueIR::Quantity { amount, .. } => Some(*amount),
        _ => None,
    }
}

fn realize_claim(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
    language: LanguageCodeIR,
) -> String {
    // A request or promise can contain asserted context alongside its scoped
    // directive/commissive content.  Rendering every contextual fact with the
    // response-wide speech act turns deadlines and owners into accidental
    // commands.  Claim modality carries the local authority, so only the
    // requested or promised content receives the imperative surface.
    if let Some(contextual_response) = action_context_surface_response(claim, response) {
        return match language {
            LanguageCodeIR::Korean => realize_korean_claim(claim, &contextual_response),
            _ => realize_english_claim(claim, &contextual_response),
        };
    }
    match language {
        LanguageCodeIR::Korean => realize_korean_claim(claim, response),
        _ => realize_english_claim(claim, response),
    }
}

/// A request or promise owns only the Action frames marked directive or
/// commissive. Its adjacent asserted facts are context, so their surface and
/// inverse both use the informational view of the same Canonical IR.
fn action_context_surface_response(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> Option<ApprovedCompositionalResponseIR> {
    let contains_scoped_action = response.claims.iter().any(|candidate| {
        candidate.relation == ApprovedRelationTypeIR::Action
            && matches!(
                candidate.modality,
                ApprovedModalityIR::Directive | ApprovedModalityIR::Commissive
            )
            && typed_action_event_node_id(candidate, response).is_some()
    });
    if !contains_scoped_action
        || !matches!(
            (response.speech_act, claim.modality),
            (ApprovedSpeechActIR::Request, ApprovedModalityIR::Asserted)
                | (ApprovedSpeechActIR::Promise, ApprovedModalityIR::Asserted)
        )
    {
        return None;
    }
    let mut contextual_response = response.clone();
    contextual_response.speech_act = ApprovedSpeechActIR::Inform;
    Some(contextual_response)
}

fn realize_korean_claim(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> String {
    let subject = claim.subject.canonical_lexical_label.trim();
    let relation = relation_label(claim.relation, LanguageCodeIR::Korean);
    let value = display_korean_claim_value(claim, response.style.register);
    let topic = object_particle(relation, "은", "는").0;
    let relation_object = object_particle(relation, "을", "를").0;
    let register = response.style.register;
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );

    if claim.relation == ApprovedRelationTypeIR::Count
        && claim.polarity
        && response.operation != ApprovedOperationIR::Negate
    {
        let count = match &claim.value {
            ApprovedOpenValueIR::Integer(amount) => Some((*amount, "개")),
            ApprovedOpenValueIR::Quantity { amount, unit } => {
                Some((*amount, unit.canonical_lexical_label.as_str()))
            }
            _ => None,
        };
        if let Some((amount, classifier)) = count {
            let clause = korean_count_speech_act_clause(
                subject,
                amount,
                classifier,
                response.speech_act,
                response.operation,
                register,
            );
            let surface = format!(
                "{clause}{}",
                if response.speech_act == ApprovedSpeechActIR::Query {
                    "?"
                } else {
                    "."
                }
            );
            return if response.speech_act == ApprovedSpeechActIR::Reassure {
                format!("{} {surface}", korean_reassurance_marker(register))
            } else {
                surface
            };
        }
    }

    if let ApprovedOpenValueIR::Boolean(value) = &claim.value {
        let asserted = if !claim.polarity || response.operation == ApprovedOperationIR::Negate {
            !*value
        } else {
            *value
        };
        if let Some(clause) = korean_boolean_speech_act_clause(
            subject,
            claim.relation,
            asserted,
            response.speech_act,
            register,
        ) {
            let surface = format!(
                "{clause}{}",
                if response.speech_act == ApprovedSpeechActIR::Query {
                    "?"
                } else {
                    "."
                }
            );
            return if response.speech_act == ApprovedSpeechActIR::Reassure {
                format!("{} {surface}", korean_reassurance_marker(register))
            } else {
                surface
            };
        }
    }

    if claim.relation == ApprovedRelationTypeIR::EarlierThan {
        if let ApprovedOpenValueIR::Lexical(reference) = &claim.value {
            let holds = claim.polarity && response.operation != ApprovedOperationIR::Negate;
            let clause = korean_earlier_than_speech_act_clause(
                subject,
                reference.canonical_lexical_label.trim(),
                holds,
                response.speech_act,
                register,
            );
            let surface = format!(
                "{clause}{}",
                if response.speech_act == ApprovedSpeechActIR::Query {
                    "?"
                } else {
                    "."
                }
            );
            return if response.speech_act == ApprovedSpeechActIR::Reassure {
                format!("{} {surface}", korean_reassurance_marker(register))
            } else {
                surface
            };
        }
    }

    if claim.polarity && response.operation != ApprovedOperationIR::Negate {
        if let ApprovedOpenValueIR::Lexical(lexical) = &claim.value {
            let subject_topic = format!("{subject}{}", object_particle(subject, "은", "는").0);
            let subject_object = format!("{subject}{}", object_particle(subject, "을", "를").0);
            let state_surface = match (claim.relation, lexical.canonical_lexical_label.as_str()) {
                (ApprovedRelationTypeIR::Status, "준비") => Some(match response.speech_act {
                    ApprovedSpeechActIR::Query => format!(
                        "{subject_topic} {}?",
                        if formal {
                            "준비됐습니까"
                        } else if casual {
                            "준비됐어"
                        } else {
                            "준비됐나요"
                        }
                    ),
                    ApprovedSpeechActIR::Request => format!(
                        "{subject_object} {}.",
                        if formal {
                            "준비해 주십시오"
                        } else if casual {
                            "준비해 줘"
                        } else {
                            "준비해 주세요"
                        }
                    ),
                    ApprovedSpeechActIR::Promise => format!(
                        "{subject_object} {}.",
                        if formal {
                            "준비하겠습니다"
                        } else if casual {
                            "준비할게"
                        } else {
                            "준비할게요"
                        }
                    ),
                    ApprovedSpeechActIR::Reassure => format!(
                        "{} {subject_topic} {}.",
                        korean_reassurance_marker(register),
                        if formal {
                            "준비됐습니다"
                        } else if casual {
                            "준비됐어"
                        } else {
                            "준비됐어요"
                        }
                    ),
                    _ => format!(
                        "{subject_topic} {}.",
                        if formal {
                            "준비됐습니다"
                        } else if casual {
                            "준비됐어"
                        } else {
                            "준비됐어요"
                        }
                    ),
                }),
                (ApprovedRelationTypeIR::Registration, "완료") => {
                    Some(match response.speech_act {
                        ApprovedSpeechActIR::Query => format!(
                            "{subject} 등록이 {}?",
                            if formal {
                                "완료됐습니까"
                            } else if casual {
                                "완료됐어"
                            } else {
                                "완료됐나요"
                            }
                        ),
                        ApprovedSpeechActIR::Request => format!(
                            "{subject} 등록을 {}.",
                            if formal {
                                "완료해 주십시오"
                            } else if casual {
                                "완료해 줘"
                            } else {
                                "완료해 주세요"
                            }
                        ),
                        ApprovedSpeechActIR::Promise => format!(
                            "{subject} 등록을 {}.",
                            if formal {
                                "완료하겠습니다"
                            } else if casual {
                                "완료할게"
                            } else {
                                "완료할게요"
                            }
                        ),
                        ApprovedSpeechActIR::Reassure => format!(
                            "{} {subject} 등록이 {}.",
                            korean_reassurance_marker(register),
                            if formal {
                                "완료됐습니다"
                            } else if casual {
                                "완료됐어"
                            } else {
                                "완료됐어요"
                            }
                        ),
                        _ => format!(
                            "{subject} 등록이 {}.",
                            if formal {
                                "완료됐습니다"
                            } else if casual {
                                "완료됐어"
                            } else {
                                "완료됐어요"
                            }
                        ),
                    })
                }
                _ => None,
            };
            if let Some(surface) = state_surface {
                return surface;
            }
        }
    }
    if claim.relation == ApprovedRelationTypeIR::Status
        && claim.polarity
        && response.operation != ApprovedOperationIR::Negate
    {
        if let Some(clause) = korean_typed_status_frame_clause(
            claim,
            response.speech_act,
            register,
        ) {
            let surface = format!(
                "{clause}{}",
                if response.speech_act == ApprovedSpeechActIR::Query {
                    "?"
                } else {
                    "."
                }
            );
            return if response.speech_act == ApprovedSpeechActIR::Reassure {
                format!("{} {surface}", korean_reassurance_marker(register))
            } else {
                surface
            };
        }
        if let Some(clause) = korean_lexical_status_predicate_clause(
            subject,
            &claim.value,
            response.speech_act,
            register,
        ) {
            let surface = format!(
                "{clause}{}",
                if response.speech_act == ApprovedSpeechActIR::Query {
                    "?"
                } else {
                    "."
                }
            );
            return if response.speech_act == ApprovedSpeechActIR::Reassure {
                format!("{} {surface}", korean_reassurance_marker(register))
            } else {
                surface
            };
        }
    }
    if claim.relation == ApprovedRelationTypeIR::Impact
        && claim.polarity
        && response.operation != ApprovedOperationIR::Negate
    {
        if let Some(clause) =
            korean_impact_need_clause(subject, &claim.value, response.speech_act, register)
        {
            let surface = format!(
                "{clause}{}",
                if response.speech_act == ApprovedSpeechActIR::Query {
                    "?"
                } else {
                    "."
                }
            );
            return if response.speech_act == ApprovedSpeechActIR::Reassure {
                format!("{} {surface}", korean_reassurance_marker(register))
            } else {
                surface
            };
        }
    }
    if claim.relation == ApprovedRelationTypeIR::Action
        && claim.polarity
        && response.operation != ApprovedOperationIR::Negate
    {
        if let Some(clause) =
            korean_action_relation_clause(subject, &claim.value, response.speech_act, register)
        {
            let surface = format!(
                "{clause}{}",
                if response.speech_act == ApprovedSpeechActIR::Query {
                    "?"
                } else {
                    "."
                }
            );
            return if response.speech_act == ApprovedSpeechActIR::Reassure {
                format!("{} {surface}", korean_reassurance_marker(register))
            } else {
                surface
            };
        }
    }
    if claim.relation == ApprovedRelationTypeIR::Location
        && claim.polarity
        && response.operation != ApprovedOperationIR::Negate
    {
        if let Some(clause) = korean_embedded_location_clause(
            subject,
            claim.subject.semantic_type,
            &claim.value,
            response.speech_act,
            register,
        ) {
            let surface = format!(
                "{clause}{}",
                if response.speech_act == ApprovedSpeechActIR::Query {
                    "?"
                } else {
                    "."
                }
            );
            return if response.speech_act == ApprovedSpeechActIR::Reassure {
                format!("{} {surface}", korean_reassurance_marker(register))
            } else {
                surface
            };
        }
    }
    if claim.relation == ApprovedRelationTypeIR::Status
        && claim.polarity
        && response.operation != ApprovedOperationIR::Negate
        && !matches!(
            response.speech_act,
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise
        )
        && matches!(
            &claim.value,
            ApprovedOpenValueIR::Lexical(_) if value.ends_with(" 상태")
        )
    {
        let ending = match response.speech_act {
            ApprovedSpeechActIR::Query if formal => "입니까?",
            ApprovedSpeechActIR::Query if casual => "야?",
            ApprovedSpeechActIR::Query => "인가요?",
            _ if formal => "입니다.",
            _ if casual => "야.",
            _ => "예요.",
        };
        let subject_topic = object_particle(subject, "은", "는").0;
        let surface = format!("{subject}{subject_topic} {value}{ending}");
        return if response.speech_act == ApprovedSpeechActIR::Reassure {
            format!("{} {surface}", korean_reassurance_marker(register))
        } else {
            surface
        };
    }
    if claim.relation == ApprovedRelationTypeIR::Status
        && claim.polarity
        && response.operation != ApprovedOperationIR::Negate
        && !matches!(
            response.speech_act,
            ApprovedSpeechActIR::Request | ApprovedSpeechActIR::Promise
        )
        && matches!(
            &claim.value,
            ApprovedOpenValueIR::Lexical(value)
                if value.canonical_lexical_label.trim_end().ends_with('중')
        )
    {
        let clause =
            korean_progress_status_clause(subject, &claim.value, response.speech_act, register)
                .expect("lexical progress status was checked above");
        let surface = format!(
            "{clause}{}",
            if response.speech_act == ApprovedSpeechActIR::Query {
                "?"
            } else {
                "."
            }
        );
        return if response.speech_act == ApprovedSpeechActIR::Reassure {
            format!("{} {surface}", korean_reassurance_marker(register))
        } else {
            surface
        };
    }
    if claim.relation == ApprovedRelationTypeIR::Status
        && claim.polarity
        && response.operation != ApprovedOperationIR::Negate
    {
        if let Some(clause) = korean_status_assignment_clause(
            subject,
            &claim.value,
            &value,
            response.speech_act,
            register,
        ) {
            return format!("{clause}.");
        }
    }
    if !matches!(claim.value, ApprovedOpenValueIR::Boolean(_))
        && claim.relation != ApprovedRelationTypeIR::EarlierThan
        && (!claim.polarity || response.operation == ApprovedOperationIR::Negate)
    {
        if let ApprovedOpenValueIR::Lexical(lexical) = &claim.value {
            if let Some(clause) = korean_negative_typed_state_speech_act_clause(
                subject,
                claim.relation,
                lexical.canonical_lexical_label.as_str(),
                response.speech_act,
                register,
            ) {
                let surface = format!(
                    "{clause}{}",
                    if response.speech_act == ApprovedSpeechActIR::Query {
                        "?"
                    } else {
                        "."
                    }
                );
                return if response.speech_act == ApprovedSpeechActIR::Reassure {
                    format!("{} {surface}", korean_reassurance_marker(register))
                } else {
                    surface
                };
            }
        }
        let clause = korean_negative_value_speech_act_clause(
            subject,
            claim.relation,
            &claim.value,
            &value,
            response.speech_act,
            register,
        );
        let surface = format!(
            "{clause}{}",
            if response.speech_act == ApprovedSpeechActIR::Query {
                "?"
            } else {
                "."
            }
        );
        return if response.speech_act == ApprovedSpeechActIR::Reassure {
            format!("{} {surface}", korean_reassurance_marker(register))
        } else {
            surface
        };
    }
    if claim.polarity && response.operation != ApprovedOperationIR::Negate {
        let ending = match response.speech_act {
            ApprovedSpeechActIR::Query => {
                if formal {
                    "입니까?"
                } else if casual {
                    return format!(
                        "{subject} {relation}{topic} {value}{}?",
                        copula_casual(particle_basis(&claim.value, &value))
                    );
                } else {
                    "인가요?"
                }
            }
            ApprovedSpeechActIR::Request => {
                let complement = directional_particle(particle_basis(&claim.value, &value)).0;
                return format!(
                    "{subject} {relation}{relation_object} {value}{complement} {}.",
                    if formal {
                        "해 주십시오"
                    } else if casual {
                        "해 줘"
                    } else {
                        "해 주세요"
                    }
                );
            }
            ApprovedSpeechActIR::Promise => {
                let complement = directional_particle(particle_basis(&claim.value, &value)).0;
                return format!(
                    "{subject} {relation}{relation_object} {value}{complement} {}.",
                    if formal {
                        "하겠습니다"
                    } else if casual {
                        "할게"
                    } else {
                        "할게요"
                    }
                );
            }
            ApprovedSpeechActIR::Reassure => {
                if formal {
                    "입니다."
                } else if casual {
                    let surface = format!(
                        "{subject} {relation}{topic} {}{}.",
                        value,
                        copula_casual(particle_basis(&claim.value, &value))
                    );
                    return format!("{} {surface}", korean_reassurance_marker(register));
                } else {
                    let surface = format!(
                        "{subject} {relation}{topic} {}{}.",
                        value,
                        copula_yo(particle_basis(&claim.value, &value))
                    );
                    return format!("{} {surface}", korean_reassurance_marker(register));
                }
            }
            _ => "",
        };
        if !ending.is_empty() {
            let surface = format!("{subject} {relation}{topic} {value}{ending}");
            return if response.speech_act == ApprovedSpeechActIR::Reassure {
                format!("{} {surface}", korean_reassurance_marker(register))
            } else {
                surface
            };
        }
    }
    if let ApprovedOpenValueIR::Boolean(value) = &claim.value {
        let asserted = if !claim.polarity || response.operation == ApprovedOperationIR::Negate {
            !*value
        } else {
            *value
        };
        return realize_korean_boolean_claim(subject, claim.relation, asserted, register);
    }
    if !claim.polarity || response.operation == ApprovedOperationIR::Negate {
        let ending = if formal {
            "아닙니다"
        } else if casual {
            "아니야"
        } else {
            "아니에요"
        };
        return format!(
            "{subject} {relation}{topic}{} {ending}.",
            object_particle(particle_basis(&claim.value, &value), "이", "가").prepend(&value)
        );
    }
    if response.operation == ApprovedOperationIR::Revise {
        let ending = if formal {
            "변경됐습니다"
        } else if casual {
            "바뀌었어"
        } else {
            "바뀌었어요"
        };
        return format!(
            "{subject} {relation}{topic}{} {ending}.",
            directional_particle(particle_basis(&claim.value, &value)).prepend(&value)
        );
    }
    let ending = if formal {
        "입니다"
    } else if casual {
        copula_casual(particle_basis(&claim.value, &value))
    } else {
        copula_yo(particle_basis(&claim.value, &value))
    };
    if let Some(relational_subject) = korean_subject_with_embedded_relation(subject, claim.relation)
    {
        let subject_topic = object_particle(&relational_subject, "은", "는").0;
        format!("{relational_subject}{subject_topic} {value}{ending}.")
    } else {
        format!("{subject} {relation}{topic} {value}{ending}.")
    }
}

fn realize_english_claim(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> String {
    let subject = claim.subject.canonical_lexical_label.trim();
    let relation = relation_label(claim.relation, LanguageCodeIR::English);
    let value = display_value(
        &claim.value,
        LanguageCodeIR::English,
        response.style.register,
    );
    if let ApprovedOpenValueIR::Boolean(value) = &claim.value {
        let asserted = if !claim.polarity || response.operation == ApprovedOperationIR::Negate {
            !*value
        } else {
            *value
        };
        return realize_english_boolean_claim(subject, claim.relation, asserted);
    }
    if !claim.polarity || response.operation == ApprovedOperationIR::Negate {
        format!("The {relation} of {subject} is not {value}.")
    } else if response.operation == ApprovedOperationIR::Revise {
        format!("The {relation} of {subject} has changed to {value}.")
    } else {
        format!("The {relation} of {subject} is {value}.")
    }
}

fn realize_korean_boolean_claim(
    subject: &str,
    relation: ApprovedRelationTypeIR,
    asserted: bool,
    register: LanguageRegisterIR,
) -> String {
    let formal = register == LanguageRegisterIR::Formal;
    let casual = matches!(
        register,
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet
    );
    let subject_topic = object_particle(subject, "은", "는").0;
    let predicate = match (relation, asserted, formal, casual) {
        (ApprovedRelationTypeIR::Cancelled, true, true, _) => "취소됐습니다",
        (ApprovedRelationTypeIR::Cancelled, true, false, false) => "취소됐어요",
        (ApprovedRelationTypeIR::Cancelled, true, false, true) => "취소됐어",
        (ApprovedRelationTypeIR::Cancelled, false, true, _) => "취소되지 않았습니다",
        (ApprovedRelationTypeIR::Cancelled, false, false, false) => "취소되지 않았어요",
        (ApprovedRelationTypeIR::Cancelled, false, false, true) => "취소되지 않았어",
        (ApprovedRelationTypeIR::RoomAvailable, true, true, _) => "사용할 수 있습니다",
        (ApprovedRelationTypeIR::RoomAvailable, true, false, false) => "사용할 수 있어요",
        (ApprovedRelationTypeIR::RoomAvailable, true, false, true) => "사용할 수 있어",
        (ApprovedRelationTypeIR::RoomAvailable, false, true, _) => "사용할 수 없습니다",
        (ApprovedRelationTypeIR::RoomAvailable, false, false, false) => "사용할 수 없어요",
        (ApprovedRelationTypeIR::RoomAvailable, false, false, true) => "사용할 수 없어",
        (ApprovedRelationTypeIR::Registration, true, true, _) => "등록됐습니다",
        (ApprovedRelationTypeIR::Registration, true, false, false) => "등록됐어요",
        (ApprovedRelationTypeIR::Registration, true, false, true) => "등록됐어",
        (ApprovedRelationTypeIR::Registration, false, true, _) => "등록되지 않았습니다",
        (ApprovedRelationTypeIR::Registration, false, false, false) => "등록되지 않았어요",
        (ApprovedRelationTypeIR::Registration, false, false, true) => "등록되지 않았어",
        (ApprovedRelationTypeIR::Entry, true, true, _) => "입장할 수 있습니다",
        (ApprovedRelationTypeIR::Entry, true, false, false) => "입장할 수 있어요",
        (ApprovedRelationTypeIR::Entry, true, false, true) => "입장할 수 있어",
        (ApprovedRelationTypeIR::Entry, false, true, _) => "입장할 수 없습니다",
        (ApprovedRelationTypeIR::Entry, false, false, false) => "입장할 수 없어요",
        (ApprovedRelationTypeIR::Entry, false, false, true) => "입장할 수 없어",
        (ApprovedRelationTypeIR::Confirmed, true, true, _) => "확정됐습니다",
        (ApprovedRelationTypeIR::Confirmed, true, false, false) => "확정됐어요",
        (ApprovedRelationTypeIR::Confirmed, true, false, true) => "확정됐어",
        (ApprovedRelationTypeIR::Confirmed, false, true, _) => "확정되지 않았습니다",
        (ApprovedRelationTypeIR::Confirmed, false, false, false) => "확정되지 않았어요",
        (ApprovedRelationTypeIR::Confirmed, false, false, true) => "확정되지 않았어",
        (ApprovedRelationTypeIR::Approved, true, true, _) => "승인됐습니다",
        (ApprovedRelationTypeIR::Approved, true, false, false) => "승인됐어요",
        (ApprovedRelationTypeIR::Approved, true, false, true) => "승인됐어",
        (ApprovedRelationTypeIR::Approved, false, true, _) => "승인되지 않았습니다",
        (ApprovedRelationTypeIR::Approved, false, false, false) => "승인되지 않았어요",
        (ApprovedRelationTypeIR::Approved, false, false, true) => "승인되지 않았어",
        _ => {
            let relation = relation_label(relation, LanguageCodeIR::Korean);
            let relation_topic = object_particle(relation, "은", "는").0;
            let truth = match (asserted, formal, casual) {
                (true, true, _) => "맞습니다",
                (true, false, false) => "맞아요",
                (true, false, true) => "맞아",
                (false, true, _) => "아닙니다",
                (false, false, false) => "아니에요",
                (false, false, true) => "아니야",
            };
            return format!("{subject} {relation}{relation_topic} {truth}.");
        }
    };
    format!("{subject}{subject_topic} {predicate}.")
}

fn realize_english_boolean_claim(
    subject: &str,
    relation: ApprovedRelationTypeIR,
    asserted: bool,
) -> String {
    let predicate = match (relation, asserted) {
        (ApprovedRelationTypeIR::Cancelled, true) => "is cancelled",
        (ApprovedRelationTypeIR::Cancelled, false) => "is not cancelled",
        (ApprovedRelationTypeIR::RoomAvailable, true) => "is available",
        (ApprovedRelationTypeIR::RoomAvailable, false) => "is not available",
        (ApprovedRelationTypeIR::Registration, true) => "is registered",
        (ApprovedRelationTypeIR::Registration, false) => "is not registered",
        (ApprovedRelationTypeIR::Entry, true) => "allows entry",
        (ApprovedRelationTypeIR::Entry, false) => "does not allow entry",
        (ApprovedRelationTypeIR::Confirmed, true) => "is confirmed",
        (ApprovedRelationTypeIR::Confirmed, false) => "is not confirmed",
        (ApprovedRelationTypeIR::Approved, true) => "is approved",
        (ApprovedRelationTypeIR::Approved, false) => "is not approved",
        _ => {
            return format!(
                "The {} of {subject} is {}.",
                relation_label(relation, LanguageCodeIR::English),
                if asserted { "true" } else { "false" }
            );
        }
    };
    format!("{subject} {predicate}.")
}

fn relation_label(relation: ApprovedRelationTypeIR, language: LanguageCodeIR) -> &'static str {
    match (language, relation) {
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Status) => "상태",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Time) => "시간",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Cancelled) => "취소 여부",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::EarlierThan) => "선행 관계",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::RoomAvailable) => "공간 사용 가능 여부",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Location) => "장소",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Registration) => "등록 상태",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Entry) => "입장 상태",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Confirmed) => "확정 여부",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Approved) => "승인 여부",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Capacity) => "정원",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Duration) => "소요 시간",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Name) => "이름",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Count) => "개수",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Date) => "날짜",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Quantity) => "수량",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Cause) => "원인",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Impact) => "영향",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Action) => "조치",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Owner) => "담당자",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Deadline) => "기한",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Agent) => "행위자",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Patient) => "수령자",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Theme) => "대상물",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Source) => "출발점",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Destination) => "도착점",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Instrument) => "도구",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Manner) => "방식",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Target) => "적용 대상",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::InitialState) => "초기 상태",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::ResultState) => "결과 상태",
        (_, ApprovedRelationTypeIR::Status) => "status",
        (_, ApprovedRelationTypeIR::Time) => "time",
        (_, ApprovedRelationTypeIR::Cancelled) => "cancellation status",
        (_, ApprovedRelationTypeIR::EarlierThan) => "ordering",
        (_, ApprovedRelationTypeIR::RoomAvailable) => "room availability",
        (_, ApprovedRelationTypeIR::Location) => "location",
        (_, ApprovedRelationTypeIR::Registration) => "registration status",
        (_, ApprovedRelationTypeIR::Entry) => "entry status",
        (_, ApprovedRelationTypeIR::Confirmed) => "confirmation status",
        (_, ApprovedRelationTypeIR::Approved) => "approval status",
        (_, ApprovedRelationTypeIR::Capacity) => "capacity",
        (_, ApprovedRelationTypeIR::Duration) => "duration",
        (_, ApprovedRelationTypeIR::Name) => "name",
        (_, ApprovedRelationTypeIR::Count) => "count",
        (_, ApprovedRelationTypeIR::Date) => "date",
        (_, ApprovedRelationTypeIR::Quantity) => "quantity",
        (_, ApprovedRelationTypeIR::Cause) => "cause",
        (_, ApprovedRelationTypeIR::Impact) => "impact",
        (_, ApprovedRelationTypeIR::Action) => "action",
        (_, ApprovedRelationTypeIR::Owner) => "owner",
        (_, ApprovedRelationTypeIR::Deadline) => "deadline",
        (_, ApprovedRelationTypeIR::Agent) => "agent",
        (_, ApprovedRelationTypeIR::Patient) => "patient",
        (_, ApprovedRelationTypeIR::Theme) => "theme",
        (_, ApprovedRelationTypeIR::Source) => "source",
        (_, ApprovedRelationTypeIR::Destination) => "destination",
        (_, ApprovedRelationTypeIR::Instrument) => "instrument",
        (_, ApprovedRelationTypeIR::Manner) => "manner",
        (_, ApprovedRelationTypeIR::Target) => "target",
        (_, ApprovedRelationTypeIR::InitialState) => "initial state",
        (_, ApprovedRelationTypeIR::ResultState) => "result state",
    }
}

fn korean_subject_with_embedded_relation(
    subject: &str,
    relation: ApprovedRelationTypeIR,
) -> Option<String> {
    let subject = subject.trim();
    // `Status` is an IR relation, not a Korean nominal head that must be
    // repeated on the surface.  The approved subject already identifies the
    // observed variable: `약탈 징후 + Status` realizes as `약탈 징후는 …`,
    // while `벌통 환기구 + Status` realizes as `벌통 환기구는 …`.
    // This leaves the relation intact for inverse parsing and avoids the
    // mechanically nominalized `… 상태는 …` form in ordinary assertions.
    if relation == ApprovedRelationTypeIR::Status {
        return Some(subject.to_string());
    }
    let relation_surface = relation_label(relation, LanguageCodeIR::Korean);
    if subject.ends_with(relation_surface) {
        return Some(subject.to_string());
    }
    match relation {
        ApprovedRelationTypeIR::Cause => Some(format!("{subject}의 원인")),
        ApprovedRelationTypeIR::Impact => Some(format!("{subject}의 영향")),
        // These heads already name the state-bearing dimension.  Adding the
        // generic relation label would produce `교체 주기 상태` or `점검 시간
        // 상태`; retain the approved subject instead.  This is a shared
        // Korean nominal grammar class, not a surface-template exception.
        ApprovedRelationTypeIR::Status
            if [
                "상태",
                "주기",
                "시간",
                "지속시간",
                "기준",
                "조건",
                "수준",
                "용량",
                "여부",
                "범위",
            ]
            .iter()
            .any(|suffix| subject.ends_with(suffix)) =>
        {
            Some(subject.to_string())
        }
        ApprovedRelationTypeIR::Owner if subject.ends_with("담당") => {
            Some(format!("{subject}자"))
        }
        ApprovedRelationTypeIR::Location
            if ["위치", "장소", "집결지", "구역", "지점"]
                .iter()
                .any(|suffix| subject.ends_with(suffix)) =>
        {
            Some(subject.to_string())
        }
        ApprovedRelationTypeIR::Source if subject.ends_with("출처") => Some(subject.to_string()),
        _ => None,
    }
}

fn display_value(
    value: &ApprovedOpenValueIR,
    language: LanguageCodeIR,
    _register: LanguageRegisterIR,
) -> String {
    match value {
        ApprovedOpenValueIR::Lexical(node) => match language {
            LanguageCodeIR::Korean => display_korean_lexical_value(&node.canonical_lexical_label),
            _ => node.canonical_lexical_label.clone(),
        },
        ApprovedOpenValueIR::Text(value) => encode_text_value(value),
        ApprovedOpenValueIR::Boolean(value) => match (language, value) {
            (LanguageCodeIR::Korean, true) => "맞음".into(),
            (LanguageCodeIR::Korean, false) => "아님".into(),
            (_, true) => "true".into(),
            (_, false) => "false".into(),
        },
        ApprovedOpenValueIR::Integer(value) => value.to_string(),
        ApprovedOpenValueIR::Clock { hour, minute } => match language {
            LanguageCodeIR::Korean => display_korean_clock(*hour, *minute),
            _ => format!("{hour:02}:{minute:02}"),
        },
        ApprovedOpenValueIR::Date { year, month, day } => match language {
            LanguageCodeIR::Korean => format!("{year}년 {month}월 {day}일"),
            _ => format!("{year:04}-{month:02}-{day:02}"),
        },
        ApprovedOpenValueIR::Quantity { amount, unit } => match language {
            LanguageCodeIR::Korean => format!("{amount}{}", unit.canonical_lexical_label),
            _ => format!("{amount} {}", unit.canonical_lexical_label),
        },
    }
}

/// Realize compact Korean state labels as ordinary noun phrases.
///
/// Canonical labels ending in `됨` are provenance-bearing state identifiers,
/// not finished Korean nominals.  Exposing them directly before a copula
/// produces forms such as `계획됨입니다`.  The surface form keeps the same
/// lexical base and makes the state nominal explicit; the inverse parser
/// accepts it only when the corresponding canonical lexical value is already
/// present in the approved response.
fn display_korean_lexical_value(label: &str) -> String {
    let label = label.trim();
    if let Some(base) = label.strip_suffix("됨").filter(|base| !base.is_empty()) {
        format!("{base}된 상태")
    } else if let Some(goal) = label
        .strip_suffix(" 도달")
        .map(str::trim_end)
        .filter(|goal| !goal.is_empty())
    {
        format!("{goal}에 도달한 상태")
    } else {
        label.to_string()
    }
}

fn display_korean_claim_value(
    claim: &ApprovedCompositionalClaimIR,
    register: LanguageRegisterIR,
) -> String {
    let value = display_value(&claim.value, LanguageCodeIR::Korean, register);
    match (&claim.value, claim.relation) {
        (ApprovedOpenValueIR::Lexical(node), ApprovedRelationTypeIR::Status) => {
            display_korean_status_value(&node.canonical_lexical_label)
        }
        (ApprovedOpenValueIR::Integer(amount), ApprovedRelationTypeIR::Capacity) => {
            format!("{amount}명")
        }
        (ApprovedOpenValueIR::Integer(amount), ApprovedRelationTypeIR::Count) => {
            format!("{amount}개")
        }
        _ => value,
    }
}

/// Expand compact event-state labels only when they occupy the typed `Status`
/// role.  The same lexical item in a location, theme, or other role remains
/// untouched.  These suffix families describe a state transition rather than
/// a style or sentence-specific paraphrase.
fn display_korean_status_value(label: &str) -> String {
    let label = label.trim();
    if let Some(state) = label
        .strip_prefix("조건부 ")
        .map(str::trim_start)
        .filter(|state| !state.is_empty())
    {
        match state {
            "완료" => "조건부로 완료된 상태".into(),
            _ => label.to_string(),
        }
    } else if let Some(base) = label
        .strip_suffix(" 가능")
        .map(str::trim_end)
        .filter(|base| !base.is_empty())
    {
        format!("{base} 가능한 상태")
    } else if let Some(base) = label
        .strip_suffix(" 확인")
        .map(str::trim_end)
        .filter(|base| !base.is_empty())
    {
        format!("{} 확인된 상태", with_particle(base, "이", "가"))
    } else if let Some(base) = label
        .strip_suffix(" 확보")
        .map(str::trim_end)
        .filter(|base| !base.is_empty())
    {
        format!("{} 확보된 상태", with_particle(base, "이", "가"))
    } else if let Some(base) = label
        .strip_suffix(" 완료")
        .map(str::trim_end)
        .filter(|base| !base.is_empty())
    {
        format!("{} 완료된 상태", with_particle(base, "이", "가"))
    } else if let Some(base) = label
        .strip_suffix(" 필요")
        .map(str::trim_end)
        .filter(|base| !base.is_empty())
    {
        format!("{} 필요한 상태", with_particle(base, "이", "가"))
    } else if let Some(base) = label
        .strip_suffix(" 안정")
        .map(str::trim_end)
        .filter(|base| !base.is_empty())
    {
        format!("{} 안정된 상태", with_particle(base, "이", "가"))
    } else {
        display_korean_lexical_value(label)
    }
}

fn encode_text_value(value: &str) -> String {
    format!(
        "“{}”",
        value
            .replace('\\', "\\\\")
            .replace('“', "\\“")
            .replace('”', "\\”")
    )
}

fn decode_text_value(surface: &str) -> Option<String> {
    let encoded = surface.strip_prefix('“')?.strip_suffix('”')?;
    let mut decoded = String::new();
    let mut escaped = false;
    for character in encoded.chars() {
        if escaped {
            decoded.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if matches!(character, '“' | '”') {
            return None;
        } else {
            decoded.push(character);
        }
    }
    (!escaped).then_some(decoded)
}

fn particle_basis<'a>(value: &'a ApprovedOpenValueIR, displayed: &'a str) -> &'a str {
    match value {
        ApprovedOpenValueIR::Text(text) => text,
        _ => displayed,
    }
}

fn copula_yo(value: &str) -> &'static str {
    if has_final_consonant(value) {
        "이에요"
    } else {
        "예요"
    }
}

fn copula_casual(value: &str) -> &'static str {
    if has_final_consonant(value) {
        "이야"
    } else {
        "야"
    }
}

fn display_korean_clock(hour: u8, minute: u8) -> String {
    let (period, display_hour) = match hour {
        0 => ("오전", 12),
        1..=11 => ("오전", hour),
        12 => ("오후", 12),
        _ => ("오후", hour - 12),
    };
    if minute == 0 {
        format!("{period} {display_hour}시")
    } else {
        format!("{period} {display_hour}시 {minute}분")
    }
}

struct Particle(&'static str);

impl Particle {
    fn prepend(&self, value: &str) -> String {
        format!(" {value}{}", self.0)
    }
}

fn object_particle(value: &str, consonant: &'static str, vowel: &'static str) -> Particle {
    if has_final_consonant(value) {
        Particle(consonant)
    } else {
        Particle(vowel)
    }
}

fn directional_particle(value: &str) -> Particle {
    if has_final_consonant(value) && !has_final_rieul(value) {
        Particle("으로")
    } else {
        Particle("로")
    }
}

fn has_final_rieul(value: &str) -> bool {
    crate::korean_nominal::surface_final_rieul(value).unwrap_or(false)
}

fn has_final_consonant(value: &str) -> bool {
    crate::korean_nominal::surface_coda(value).unwrap_or(false)
}

fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn mermaid_text(value: &str) -> String {
    value.replace(['"', '[', ']'], " ")
}

fn parse_ordered_item(line: &str) -> Option<(usize, &str)> {
    let (number, surface) = line.split_once(". ")?;
    let ordinal = number.parse::<usize>().ok()?;
    (!surface.trim().is_empty()).then_some((ordinal, surface.trim()))
}

fn push_surface_node(
    nodes: &mut Vec<DocumentSurfaceNodeIR>,
    kind: DocumentSurfaceNodeKindIR,
    ordinal: Option<usize>,
    surface: &str,
) {
    nodes.push(DocumentSurfaceNodeIR {
        node_index: nodes.len(),
        kind,
        ordinal,
        surface: surface.trim().to_string(),
    });
}

fn sha256_json<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("document response IR serializes"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approved_response::{
        compositional_response_sha256, ApprovedActionEventArgumentIR, ApprovedActionEventFrameIR,
        ApprovedClauseUnitIR, ApprovedEventDiscourseStateIR,
        ApprovedEventExpressionPreferenceIR, ApprovedEventPragmaticContextIR,
        ApprovedLexicalNodeIR, ApprovedModalityIR, ApprovedResponseStyleIR, ApprovedSemanticTypeIR,
        ApprovedSpeechActIR, APPROVED_EVENT_DISCOURSE_STATE_SCHEMA,
        APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA, COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA,
    };
    use crate::discourse_focus::{DiscourseFocusCandidateIR, DiscourseFocusStateIR};

    fn claim(
        id: &str,
        subject_id: &str,
        subject: &str,
        relation: ApprovedRelationTypeIR,
        value: ApprovedOpenValueIR,
    ) -> ApprovedCompositionalClaimIR {
        ApprovedCompositionalClaimIR {
            proposition_id: id.into(),
            subject: ApprovedLexicalNodeIR {
                node_id: subject_id.into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: subject.into(),
            },
            relation,
            value,
            polarity: true,
            modality: ApprovedModalityIR::Asserted,
                    status_frame: None,
}
}

    fn response(
        relation: ApprovedDiscourseRelationIR,
        operation: ApprovedOperationIR,
        claims: Vec<ApprovedCompositionalClaimIR>,
        verbosity: ApprovedVerbosityIR,
    ) -> ApprovedCompositionalResponseIR {
        let mut response = ApprovedCompositionalResponseIR {
            schema: COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA.into(),
            speech_act: if verbosity == ApprovedVerbosityIR::Short {
                ApprovedSpeechActIR::Inform
            } else {
                ApprovedSpeechActIR::Explain
            },
            operation,
            clause_plan: claims
                .iter()
                .enumerate()
                .map(|(index, claim)| ApprovedClauseUnitIR {
                    unit_index: index,
                    role: "ASSERT".into(),
                    proposition_ids: vec![claim.proposition_id.clone()],
                    predecessor_indices: (index > 0).then(|| index - 1).into_iter().collect(),
                })
                .collect(),
            claims,
            event_realizations: Vec::new(),
            discourse_relation: relation,
            style: ApprovedResponseStyleIR {
                register: LanguageRegisterIR::Formal,
                verbosity,
            },
            source_world_state_sha256: "a".repeat(64),
            source_deliberation_sha256: "b".repeat(64),
            approval_replay_verified: true,
            unsupported_claims: 0,
            semantic_sha256: String::new(),
        };
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        response
    }

    #[test]
    fn compact_single_status_surface_roundtrips_without_relation_label() {
        let response = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_STATUS_COMPACT",
                "meeting",
                "회의",
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "prepared".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "준비".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        let parsed =
            interpret_document_semantics("회의는 준비입니다.", &response, LanguageCodeIR::Korean)
                .expect("compact status should parse");
        assert!(parsed.validate(&response));
    }

    #[test]
    fn typed_within_range_status_uses_range_compliance_not_possession_language() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_PRESSURE_RANGE",
                "negative_pressure",
                "음압 격리실 압력",
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "permitted_range_secured".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "허용 범위 확보".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.claims[0].status_frame = Some(ApprovedStatusFrameIR::WithinRange {
            range: ApprovedLexicalNodeIR {
                node_id: "negative_pressure_permitted_range".into(),
                semantic_type: ApprovedSemanticTypeIR::State,
                canonical_lexical_label: "허용 범위".into(),
            },
        });
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        assert!(approved.validate());

        let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
        assert_eq!(output.markdown, "음압 격리실 압력은 허용 범위 안에 있습니다.");
        assert!(!output.markdown.contains("허용 범위가 확보"));
        assert!(output.validate(&approved));
        assert_eq!(output.unsupported_claims, 0);
    }

    #[test]
    fn typed_capacity_and_verification_frames_preserve_predicate_structure() {
        let cases = [
            (
                "backup_battery",
                "비상 배터리",
                "정격 용량 확보",
                ApprovedStatusFrameIR::CapacitySufficient {
                    criterion: ApprovedLexicalNodeIR {
                        node_id: "rated_capacity".into(),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: "정격 용량".into(),
                    },
                },
                "비상 배터리는 정격 용량을 충족합니다.",
            ),
            (
                "satellite_phone_test",
                "위성 전화 시험",
                "통화 품질 확인",
                ApprovedStatusFrameIR::Verification {
                    finding: ApprovedLexicalNodeIR {
                        node_id: "call_quality".into(),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: "통화 품질".into(),
                    },
                },
                "위성 전화 시험에서 통화 품질이 확인됐습니다.",
            ),
        ];
        for (subject_id, subject, legacy_value, status_frame, expected) in cases {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    &format!("P_{subject_id}"),
                    subject_id,
                    subject,
                    ApprovedRelationTypeIR::Status,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: format!("{subject_id}_legacy_value"),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: legacy_value.into(),
                    }),
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.claims[0].status_frame = Some(status_frame);
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            assert!(approved.validate());

            let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
            assert_eq!(output.markdown, expected);
            assert!(output.validate(&approved));
            assert_eq!(output.unsupported_claims, 0);
        }
    }

    fn with_event_realization(
        mut response: ApprovedCompositionalResponseIR,
        subject_node_id: &str,
        class: ApprovedEventRealizationClassIR,
        phase: ApprovedEventPhaseIR,
    ) -> ApprovedCompositionalResponseIR {
        response
            .event_realizations
            .push(crate::approved_response::ApprovedEventRealizationIR {
                subject_node_id: subject_node_id.into(),
                class,
                predicate_sense: None,
                phase: Some(phase),
                perspective: None,
                voice: None,
                information_structure: None,
            });
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        response
    }

    fn with_transfer_perspective(
        mut response: ApprovedCompositionalResponseIR,
        perspective: Option<ApprovedEventPerspectiveIR>,
    ) -> ApprovedCompositionalResponseIR {
        response
            .event_realizations
            .push(crate::approved_response::ApprovedEventRealizationIR {
                subject_node_id: "transfer".into(),
                class: ApprovedEventRealizationClassIR::Transfer,
                predicate_sense: Some(ApprovedEventPredicateSenseIR::Transfer),
                phase: Some(ApprovedEventPhaseIR::Completed),
                perspective,
                voice: None,
                information_structure: None,
            });
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        response
    }

    fn with_semantic_event_realization(
        mut response: ApprovedCompositionalResponseIR,
        subject_node_id: &str,
        class: ApprovedEventRealizationClassIR,
        predicate_sense: Option<ApprovedEventPredicateSenseIR>,
        perspective: Option<ApprovedEventPerspectiveIR>,
    ) -> ApprovedCompositionalResponseIR {
        response
            .event_realizations
            .push(crate::approved_response::ApprovedEventRealizationIR {
                subject_node_id: subject_node_id.into(),
                class,
                predicate_sense,
                phase: Some(ApprovedEventPhaseIR::Completed),
                perspective,
                voice: None,
                information_structure: None,
            });
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        response
    }

    fn with_transfer_information_structure(
        mut response: ApprovedCompositionalResponseIR,
        voice: ApprovedEventVoiceIR,
        information_structure: ApprovedEventInformationStructureIR,
    ) -> ApprovedCompositionalResponseIR {
        response
            .event_realizations
            .push(crate::approved_response::ApprovedEventRealizationIR {
                subject_node_id: "transfer".into(),
                class: ApprovedEventRealizationClassIR::Transfer,
                predicate_sense: Some(ApprovedEventPredicateSenseIR::Transfer),
                phase: Some(ApprovedEventPhaseIR::Completed),
                perspective: None,
                voice: Some(voice),
                information_structure: Some(information_structure),
            });
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        response
    }

    fn transfer_claims() -> Vec<ApprovedCompositionalClaimIR> {
        vec![
            claim(
                "P_AGENT",
                "transfer",
                "전송",
                ApprovedRelationTypeIR::Agent,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "minsu".into(),
                    semantic_type: ApprovedSemanticTypeIR::Person,
                    canonical_lexical_label: "민수".into(),
                }),
            ),
            claim(
                "P_THEME",
                "transfer",
                "전송",
                ApprovedRelationTypeIR::Theme,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "report".into(),
                    semantic_type: ApprovedSemanticTypeIR::Concept,
                    canonical_lexical_label: "보고서".into(),
                }),
            ),
            claim(
                "P_DESTINATION",
                "transfer",
                "전송",
                ApprovedRelationTypeIR::Destination,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "server".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "서버".into(),
                }),
            ),
        ]
    }

    #[test]
    fn approved_meaning_becomes_a_hierarchical_korean_document_without_new_claims() {
        let response = response(
            ApprovedDiscourseRelationIR::Explanation,
            ApprovedOperationIR::Explain,
            vec![
                claim(
                    "P_TIME",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Time,
                    ApprovedOpenValueIR::Clock {
                        hour: 16,
                        minute: 0,
                    },
                ),
                claim(
                    "P_LOCATION",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Location,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "room_b".into(),
                        semantic_type: ApprovedSemanticTypeIR::Location,
                        canonical_lexical_label: "회의실 B".into(),
                    }),
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );
        let output = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(output.validate(&response));
        assert!(output.markdown.contains("## 이유와 설명"));
        assert!(output.markdown.contains("회의 시간은 오후 4시입니다."));
        assert!(output.markdown.contains("정리하면, 장소는 회의실 B입니다."));
        assert!(!output.markdown.contains("P_TIME"));
        assert_eq!(output.unsupported_claims, 0);
        assert_eq!(
            output.semantic_interpretation.recovered_claim_ids,
            vec!["P_TIME", "P_LOCATION"]
        );
        assert!(output
            .interpreted_structure
            .nodes
            .iter()
            .any(|node| node.kind == DocumentSurfaceNodeKindIR::Paragraph));
        assert!(!output.markdown.contains("1. 회의 시간"));
    }

    #[test]
    fn unseen_long_discourse_preserves_register_numbering_and_semantic_inverse() {
        let mut neutral = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![
                claim(
                    "P_CAMP_NAME",
                    "camping",
                    "캠핑",
                    ApprovedRelationTypeIR::Name,
                    ApprovedOpenValueIR::Text("주말 캠핑".into()),
                ),
                claim(
                    "P_CAMP_CONFIRMED",
                    "camping",
                    "캠핑",
                    ApprovedRelationTypeIR::Confirmed,
                    ApprovedOpenValueIR::Boolean(true),
                ),
                claim(
                    "P_CAMP_DATE",
                    "camping",
                    "캠핑",
                    ApprovedRelationTypeIR::Date,
                    ApprovedOpenValueIR::Date {
                        year: 2026,
                        month: 11,
                        day: 7,
                    },
                ),
                claim(
                    "P_CAMP_TIME",
                    "camping",
                    "캠핑",
                    ApprovedRelationTypeIR::Time,
                    ApprovedOpenValueIR::Clock {
                        hour: 10,
                        minute: 0,
                    },
                ),
                claim(
                    "P_CAMP_LOCATION",
                    "camping",
                    "캠핑",
                    ApprovedRelationTypeIR::Location,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "riverside_camp".into(),
                        semantic_type: ApprovedSemanticTypeIR::Location,
                        canonical_lexical_label: "강변 야영장".into(),
                    }),
                ),
                claim(
                    "P_CAMP_ENTRY",
                    "camping",
                    "캠핑",
                    ApprovedRelationTypeIR::Entry,
                    ApprovedOpenValueIR::Boolean(true),
                ),
                claim(
                    "P_CAMP_CAPACITY",
                    "camping",
                    "캠핑",
                    ApprovedRelationTypeIR::Capacity,
                    ApprovedOpenValueIR::Integer(6),
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );
        neutral.style.register = LanguageRegisterIR::Neutral;
        neutral.semantic_sha256 = compositional_response_sha256(&neutral);
        assert!(neutral.validate());

        let neutral_output = realize_document_response(&neutral, LanguageCodeIR::Korean).unwrap();
        assert!(neutral_output.validate(&neutral));
        assert!(neutral_output
            .markdown
            .starts_with("핵심 내용을 항목별로 정리했어요."));
        assert!(neutral_output.markdown.contains("## 핵심 내용"));
        for heading in [
            "### 1. 핵심 내용",
            "### 2. 시간 정보",
            "### 3. 장소와 이용",
            "### 4. 수량과 규모",
        ] {
            assert!(neutral_output.markdown.contains(heading), "{heading}");
        }
        for surface in [
            "캠핑 이름은 “주말 캠핑”이고, 확정됐어요.",
            "날짜는 2026년 11월 7일",
            "시간은 오전 10시예요.",
            "장소는 강변 야영장이고, 입장할 수 있어요.",
            "캠핑 정원은 6명이에요.",
        ] {
            assert!(
                neutral_output.markdown.contains(surface),
                "missing `{surface}` in:\n{}",
                neutral_output.markdown
            );
        }

        let mut informal = neutral.clone();
        informal.style.register = LanguageRegisterIR::Informal;
        informal.semantic_sha256 = compositional_response_sha256(&informal);
        assert!(informal.validate());
        let informal_output = realize_document_response(&informal, LanguageCodeIR::Korean).unwrap();
        assert!(informal_output.validate(&informal));
        assert!(informal_output
            .markdown
            .starts_with("핵심 내용을 항목별로 정리했어."));
        for surface in [
            "캠핑 이름은 “주말 캠핑”이고, 확정됐어.",
            "시간은 오전 10시야.",
            "장소는 강변 야영장이고, 입장할 수 있어.",
            "캠핑 정원은 6명이야.",
            "위 내용은 확인된 정보만 반영했어.",
        ] {
            assert!(
                informal_output.markdown.contains(surface),
                "missing `{surface}` in:\n{}",
                informal_output.markdown
            );
        }
        assert!(!informal_output.markdown.contains("습니다"));
        assert!(!informal_output.markdown.contains("어요"));
        assert_eq!(
            informal_output.semantic_interpretation.recovered_claim_ids,
            informal
                .claims
                .iter()
                .map(|claim| claim.proposition_id.clone())
                .collect::<Vec<_>>()
        );
        assert_ne!(neutral_output.markdown, informal_output.markdown);
        println!("LONG_DISCOURSE_NEUTRAL\n{}", neutral_output.markdown);
        println!("LONG_DISCOURSE_INFORMAL\n{}", informal_output.markdown);
    }

    #[test]
    fn causal_support_and_conclusion_are_planned_and_roundtrip() {
        let response = response(
            ApprovedDiscourseRelationIR::Cause,
            ApprovedOperationIR::Explain,
            vec![
                claim(
                    "P_STATUS",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Status,
                    ApprovedOpenValueIR::Text("준비 중".into()),
                ),
                claim(
                    "P_CANCELLED",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Cancelled,
                    ApprovedOpenValueIR::Boolean(true),
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );
        let output = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(output.validate(&response));
        assert_eq!(
            output
                .plan
                .rhetorical_units
                .iter()
                .map(|unit| unit.rhetorical_move)
                .collect::<Vec<_>>(),
            vec![
                DocumentRhetoricalMoveIR::Cause,
                DocumentRhetoricalMoveIR::Conclusion,
            ]
        );
        assert!(output.plan.clause_fusions.is_empty());
        assert!(output
            .markdown
            .contains("회의는 “준비 중”입니다. 따라서, 취소됐습니다."));

        let english = realize_document_response(&response, LanguageCodeIR::English).unwrap();
        assert!(english.validate(&response));
        assert!(english
            .markdown
            .contains("The status of 회의 is “준비 중”. Therefore, it is cancelled."));

        let wrong_relation = output.markdown.replace("따라서,", "정리하면,");
        assert!(
            interpret_document_semantics(&wrong_relation, &response, LanguageCodeIR::Korean,)
                .is_err()
        );

        let mut tampered = output.plan.clone();
        tampered.rhetorical_units[0].rhetorical_move = DocumentRhetoricalMoveIR::Support;
        tampered.plan_sha256 = document_response_plan_sha256(&tampered);
        assert!(!tampered.validate(&response));
    }

    #[test]
    fn conditional_conclusion_is_explicit_and_semantically_reversible() {
        let response = response(
            ApprovedDiscourseRelationIR::Condition,
            ApprovedOperationIR::Condition,
            vec![
                claim(
                    "P_REGISTERED",
                    "event",
                    "행사",
                    ApprovedRelationTypeIR::Registration,
                    ApprovedOpenValueIR::Boolean(true),
                ),
                claim(
                    "P_ENTRY",
                    "event",
                    "행사",
                    ApprovedRelationTypeIR::Entry,
                    ApprovedOpenValueIR::Boolean(true),
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );
        let korean = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(korean.validate(&response));
        assert!(korean.plan.clause_fusions.is_empty());
        assert!(korean
            .markdown
            .contains("행사는 등록됐습니다. 이 조건이 충족되면, 입장할 수 있습니다."));

        let english = realize_document_response(&response, LanguageCodeIR::English).unwrap();
        assert!(english.validate(&response));
        assert!(english
            .markdown
            .contains("행사 is registered. If that condition is met, it allows entry."));
    }

    #[test]
    fn simultaneous_relation_is_explicit_and_semantically_reversible() {
        let response = response(
            ApprovedDiscourseRelationIR::Simultaneous,
            ApprovedOperationIR::Assert,
            vec![
                claim(
                    "P_STATUS_SIMULTANEOUS",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Status,
                    ApprovedOpenValueIR::Text("준비 중".into()),
                ),
                claim(
                    "P_CANCELLED_SIMULTANEOUS",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Cancelled,
                    ApprovedOpenValueIR::Boolean(true),
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );
        let korean = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(korean.validate(&response));
        assert_eq!(
            korean
                .plan
                .rhetorical_units
                .iter()
                .map(|unit| unit.rhetorical_move)
                .collect::<Vec<_>>(),
            vec![
                DocumentRhetoricalMoveIR::Assertion,
                DocumentRhetoricalMoveIR::Concurrent,
            ]
        );
        assert!(korean
            .markdown
            .contains("회의는 “준비 중”입니다. 동시에, 취소됐습니다."));

        let english = realize_document_response(&response, LanguageCodeIR::English).unwrap();
        assert!(english.validate(&response));
        assert!(english
            .markdown
            .contains("The status of 회의 is “준비 중”. At the same time, it is cancelled."));

        let wrong_relation = korean.markdown.replace("동시에,", "따라서,");
        assert!(
            interpret_document_semantics(&wrong_relation, &response, LanguageCodeIR::Korean)
                .is_err()
        );
    }

    #[test]
    fn correction_frame_is_used_once_and_cannot_cross_discourse_types() {
        let correction_response = response(
            ApprovedDiscourseRelationIR::Correction,
            ApprovedOperationIR::Revise,
            vec![
                claim(
                    "P_TIME",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Time,
                    ApprovedOpenValueIR::Clock {
                        hour: 16,
                        minute: 0,
                    },
                ),
                claim(
                    "P_LOCATION",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Location,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "room_b".into(),
                        semantic_type: ApprovedSemanticTypeIR::Location,
                        canonical_lexical_label: "회의실 B".into(),
                    }),
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );
        let korean =
            realize_document_response(&correction_response, LanguageCodeIR::Korean).unwrap();
        assert!(korean.validate(&correction_response));
        assert!(korean.markdown.contains(
            "정정된 내용은 다음과 같습니다. 회의 시간은 오후 4시로 변경됐고, 장소는 회의실 B로 변경됐습니다."
        ));
        assert_eq!(
            korean
                .markdown
                .matches("정정된 내용은 다음과 같습니다.")
                .count(),
            1
        );

        let english =
            realize_document_response(&correction_response, LanguageCodeIR::English).unwrap();
        assert!(english.validate(&correction_response));
        assert!(english.markdown.contains(
            "Correction: The time of 회의 has changed to 16:00, and its location has changed to 회의실 B."
        ));

        let statement = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_TIME",
                "meeting",
                "회의",
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 0,
                },
            )],
            ApprovedVerbosityIR::Short,
        );
        assert!(interpret_document_semantics(
            "정정된 내용은 다음과 같습니다. 회의 시간은 오후 4시입니다.",
            &statement,
            LanguageCodeIR::Korean,
        )
        .is_err());
    }

    #[test]
    fn typed_clause_fusion_is_bounded_reversible_and_plan_sealed() {
        let response = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![
                claim(
                    "P_STATUS",
                    "event",
                    "행사",
                    ApprovedRelationTypeIR::Status,
                    ApprovedOpenValueIR::Text("준비 중이고, 확인됨".into()),
                ),
                claim(
                    "P_CANCELLED",
                    "event",
                    "행사",
                    ApprovedRelationTypeIR::Cancelled,
                    ApprovedOpenValueIR::Boolean(false),
                ),
                claim(
                    "P_REGISTERED",
                    "event",
                    "행사",
                    ApprovedRelationTypeIR::Registration,
                    ApprovedOpenValueIR::Boolean(true),
                ),
                claim(
                    "P_CONFIRMED",
                    "event",
                    "행사",
                    ApprovedRelationTypeIR::Confirmed,
                    ApprovedOpenValueIR::Boolean(true),
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );
        let korean = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(korean.validate(&response));
        assert_eq!(korean.plan.clause_fusions.len(), MAX_FUSED_CLAUSES - 1);
        assert_eq!(
            korean
                .plan
                .clause_fusions
                .iter()
                .map(|fusion| (
                    fusion.left_proposition_id.as_str(),
                    fusion.right_proposition_id.as_str(),
                ))
                .collect::<Vec<_>>(),
            vec![("P_STATUS", "P_CANCELLED"), ("P_CANCELLED", "P_REGISTERED"),]
        );
        assert!(korean.markdown.contains(
            "행사는 “준비 중이고, 확인됨”이고, 취소되지 않았고, 등록됐습니다. 또한, 확정됐습니다."
        ));
        assert_eq!(
            korean.semantic_interpretation.recovered_claim_ids,
            vec!["P_STATUS", "P_CANCELLED", "P_REGISTERED", "P_CONFIRMED"]
        );

        let english = realize_document_response(&response, LanguageCodeIR::English).unwrap();
        assert!(english.validate(&response));
        assert!(english.markdown.contains(
            "The status of 행사 is “준비 중이고, 확인됨”, and it is not cancelled, and it is registered. Also, it is confirmed."
        ));

        let mut tampered = korean.plan.clone();
        tampered.clause_fusions[0].right_proposition_id = "P_CONFIRMED".into();
        tampered.plan_sha256 = document_response_plan_sha256(&tampered);
        assert!(!tampered.validate(&response));
    }

    #[test]
    fn typed_event_predicate_realizes_and_recovers_schedule_arguments() {
        let response = with_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![
                    claim(
                        "P_DATE",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 10,
                            day: 12,
                        },
                    ),
                    claim(
                        "P_TIME",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 16,
                            minute: 30,
                        },
                    ),
                    claim(
                        "P_LOCATION",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Location,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "hall_b".into(),
                            semantic_type: ApprovedSemanticTypeIR::Location,
                            canonical_lexical_label: "B홀".into(),
                        }),
                    ),
                ],
                ApprovedVerbosityIR::Explanatory,
            ),
            "event",
            ApprovedEventRealizationClassIR::HostedEvent,
            ApprovedEventPhaseIR::Scheduled,
        );

        let korean = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(korean.validate(&response));
        assert_eq!(korean.plan.event_predicates.len(), 1);
        assert!(korean.plan.clause_fusions.is_empty());
        assert_eq!(
            korean.plan.event_predicates[0]
                .arguments
                .iter()
                .map(|argument| (argument.proposition_id.as_str(), argument.role,))
                .collect::<Vec<_>>(),
            vec![
                ("P_DATE", DocumentEventArgumentRoleIR::Date),
                ("P_TIME", DocumentEventArgumentRoleIR::Time),
                ("P_LOCATION", DocumentEventArgumentRoleIR::Location),
            ]
        );
        assert!(korean
            .markdown
            .contains("행사는 2026년 10월 12일 오후 4시 30분에 B홀에서 열립니다."));
        assert_eq!(
            korean.semantic_interpretation.recovered_claim_ids,
            vec!["P_DATE", "P_TIME", "P_LOCATION"]
        );

        let english = realize_document_response(&response, LanguageCodeIR::English).unwrap();
        assert!(english.validate(&response));
        assert!(english
            .markdown
            .contains("행사 takes place on 2026-10-12 at 16:30 in B홀."));

        let mut tampered = korean.plan.clone();
        tampered.event_predicates[0].arguments[0].role = DocumentEventArgumentRoleIR::Location;
        tampered.plan_sha256 = document_response_plan_sha256(&tampered);
        assert!(!tampered.validate(&response));

        assert!(interpret_document_semantics(
            "행사는 2026년 10월 12일 오후 5시에 B홀에서 열립니다.",
            &response,
            LanguageCodeIR::Korean,
        )
        .is_err());
    }

    #[test]
    fn typed_transfer_frame_changes_voice_and_focus_without_changing_claims() {
        let base = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            transfer_claims(),
            ApprovedVerbosityIR::Short,
        );
        let cases = [
            (
                ApprovedEventPerspectiveIR {
                    voice: ApprovedEventVoiceIR::Active,
                    focus: ApprovedEventFocusIR::Agent,
                },
                "민수가 보고서를 서버로 전송했습니다.",
                "민수 transferred 보고서 to 서버.",
            ),
            (
                ApprovedEventPerspectiveIR {
                    voice: ApprovedEventVoiceIR::Active,
                    focus: ApprovedEventFocusIR::Theme,
                },
                "보고서는 민수가 서버로 전송했습니다.",
                "As for 보고서, 민수 transferred it to 서버.",
            ),
            (
                ApprovedEventPerspectiveIR {
                    voice: ApprovedEventVoiceIR::Active,
                    focus: ApprovedEventFocusIR::Destination,
                },
                "서버에는 민수가 보고서를 전송했습니다.",
                "To 서버, 민수 transferred 보고서.",
            ),
            (
                ApprovedEventPerspectiveIR {
                    voice: ApprovedEventVoiceIR::Passive,
                    focus: ApprovedEventFocusIR::Theme,
                },
                "보고서는 민수에 의해 서버로 전송됐습니다.",
                "보고서 was transferred by 민수 to 서버.",
            ),
        ];

        for (perspective, korean_surface, english_surface) in cases {
            let approved = with_transfer_perspective(base.clone(), Some(perspective));
            let korean = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
            assert!(korean.validate(&approved));
            assert!(korean.markdown.contains(korean_surface));
            assert_eq!(korean.plan.event_predicates.len(), 1);
            assert_eq!(
                korean.plan.sections[0].role,
                DocumentResponseRoleIR::EventFrame
            );
            assert_eq!(
                korean.plan.event_predicates[0].perspective,
                Some(perspective)
            );
            assert_eq!(
                korean.semantic_interpretation.recovered_claim_ids,
                vec!["P_AGENT", "P_THEME", "P_DESTINATION"]
            );

            let english = realize_document_response(&approved, LanguageCodeIR::English).unwrap();
            assert!(english.validate(&approved));
            assert!(english.markdown.contains(english_surface));
        }

        let approved = with_transfer_perspective(
            base,
            Some(ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            }),
        );
        let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
        let mut tampered = output.plan.clone();
        tampered.event_predicates[0].perspective = Some(ApprovedEventPerspectiveIR {
            voice: ApprovedEventVoiceIR::Passive,
            focus: ApprovedEventFocusIR::Theme,
        });
        tampered.plan_sha256 = document_response_plan_sha256(&tampered);
        assert!(!tampered.validate(&approved));
        assert!(interpret_document_semantics(
            "민수가 보고서를 다른 서버로 전송했습니다.",
            &approved,
            LanguageCodeIR::Korean,
        )
        .is_err());
    }

    #[test]
    fn typed_information_structure_realizes_four_surfaces_from_one_event_frame() {
        let base = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            transfer_claims(),
            ApprovedVerbosityIR::Short,
        );
        let cases = [
            (
                ApprovedEventVoiceIR::Active,
                ApprovedEventInformationStructureIR {
                    topic: None,
                    focus: ApprovedEventInformationRoleIR::Agent,
                    omitted_roles: Vec::new(),
                    recoverable_roles: Vec::new(),
                },
                "민수가 보고서를 서버로 전송했습니다.",
                "민수 transferred 보고서 to 서버.",
            ),
            (
                ApprovedEventVoiceIR::Active,
                ApprovedEventInformationStructureIR {
                    topic: Some(ApprovedEventInformationRoleIR::Theme),
                    focus: ApprovedEventInformationRoleIR::Agent,
                    omitted_roles: Vec::new(),
                    recoverable_roles: Vec::new(),
                },
                "보고서는 민수가 서버로 전송했습니다.",
                "As for 보고서, 민수 transferred it to 서버.",
            ),
            (
                ApprovedEventVoiceIR::Passive,
                ApprovedEventInformationStructureIR {
                    topic: None,
                    focus: ApprovedEventInformationRoleIR::Theme,
                    omitted_roles: vec![ApprovedRelationTypeIR::Agent],
                    recoverable_roles: vec![ApprovedRelationTypeIR::Agent],
                },
                "보고서가 서버로 전송됐습니다.",
                "보고서 was transferred to 서버.",
            ),
            (
                ApprovedEventVoiceIR::Passive,
                ApprovedEventInformationStructureIR {
                    topic: Some(ApprovedEventInformationRoleIR::Destination),
                    focus: ApprovedEventInformationRoleIR::Theme,
                    omitted_roles: vec![ApprovedRelationTypeIR::Agent],
                    recoverable_roles: vec![ApprovedRelationTypeIR::Agent],
                },
                "서버에는 보고서가 전송됐습니다.",
                "To 서버, 보고서 was transferred.",
            ),
        ];
        let expected_claim_ids = vec!["P_AGENT", "P_THEME", "P_DESTINATION"];
        let mut recovered_frames = Vec::new();

        for (voice, information, korean_surface, english_surface) in cases {
            let approved =
                with_transfer_information_structure(base.clone(), voice, information.clone());
            let korean = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
            assert!(korean.validate(&approved));
            assert!(korean.markdown.contains(korean_surface));
            assert_eq!(
                korean.semantic_interpretation.recovered_claim_ids,
                expected_claim_ids
            );
            assert_eq!(korean.plan.event_predicates[0].voice, Some(voice));
            assert_eq!(
                korean.plan.event_predicates[0].information_structure,
                Some(information.clone())
            );
            recovered_frames.push(
                korean
                    .semantic_interpretation
                    .claims
                    .iter()
                    .map(|claim| {
                        (
                            claim.proposition_id.clone(),
                            claim.subject.clone(),
                            claim.relation,
                            claim.value.clone(),
                            claim.polarity,
                            claim.modality,
                        )
                    })
                    .collect::<Vec<_>>(),
            );

            let english = realize_document_response(&approved, LanguageCodeIR::English).unwrap();
            assert!(english.validate(&approved));
            assert!(english.markdown.contains(english_surface));
            assert_eq!(
                english.semantic_interpretation.recovered_claim_ids,
                expected_claim_ids
            );
        }
        assert!(recovered_frames
            .windows(2)
            .all(|frames| frames[0] == frames[1]));
    }

    #[test]
    fn typed_discourse_state_automatically_selects_four_information_structures() {
        let base = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                transfer_claims(),
                ApprovedVerbosityIR::Short,
            ),
            "transfer",
            ApprovedEventRealizationClassIR::Transfer,
            Some(ApprovedEventPredicateSenseIR::Transfer),
            None,
        );
        let state = |current_topic,
                     requested_focus,
                     backgrounded_roles: Vec<ApprovedRelationTypeIR>,
                     recoverable_roles: Vec<ApprovedRelationTypeIR>| {
            ApprovedEventDiscourseStateIR {
                schema: APPROVED_EVENT_DISCOURSE_STATE_SCHEMA.into(),
                subject_node_id: "transfer".into(),
                current_topic,
                requested_focus,
                backgrounded_roles,
                recoverable_roles,
            }
        };
        let cases = [
            (
                state(
                    None,
                    ApprovedEventInformationRoleIR::Event,
                    Vec::new(),
                    Vec::new(),
                ),
                ApprovedEventVoiceIR::Active,
                None,
                ApprovedEventInformationRoleIR::Event,
                "민수가 보고서를 서버로 전송했습니다.",
            ),
            (
                state(
                    Some(ApprovedEventInformationRoleIR::Theme),
                    ApprovedEventInformationRoleIR::Theme,
                    Vec::new(),
                    Vec::new(),
                ),
                ApprovedEventVoiceIR::Active,
                Some(ApprovedEventInformationRoleIR::Theme),
                ApprovedEventInformationRoleIR::Agent,
                "보고서는 민수가 서버로 전송했습니다.",
            ),
            (
                state(
                    None,
                    ApprovedEventInformationRoleIR::Theme,
                    vec![ApprovedRelationTypeIR::Agent],
                    vec![ApprovedRelationTypeIR::Agent],
                ),
                ApprovedEventVoiceIR::Passive,
                None,
                ApprovedEventInformationRoleIR::Theme,
                "보고서가 서버로 전송됐습니다.",
            ),
            (
                state(
                    Some(ApprovedEventInformationRoleIR::Destination),
                    ApprovedEventInformationRoleIR::Destination,
                    vec![ApprovedRelationTypeIR::Agent],
                    vec![ApprovedRelationTypeIR::Agent],
                ),
                ApprovedEventVoiceIR::Passive,
                Some(ApprovedEventInformationRoleIR::Destination),
                ApprovedEventInformationRoleIR::Theme,
                "서버에는 보고서가 전송됐습니다.",
            ),
        ];
        let mut recovered_frames = Vec::new();

        for (state, voice, topic, focus, surface) in cases {
            let projected = base.apply_event_discourse_states(&[state]).unwrap();
            let realization = &projected.event_realizations[0];
            assert_eq!(realization.voice, Some(voice));
            assert_eq!(
                realization
                    .information_structure
                    .as_ref()
                    .map(|information| (information.topic, information.focus)),
                Some((topic, focus))
            );
            let output = realize_document_response(&projected, LanguageCodeIR::Korean).unwrap();
            assert!(output.validate(&projected));
            assert!(output.markdown.contains(surface), "{}", output.markdown);
            recovered_frames.push(
                output
                    .semantic_interpretation
                    .claims
                    .iter()
                    .map(|claim| {
                        (
                            claim.proposition_id.clone(),
                            claim.subject.clone(),
                            claim.relation,
                            claim.value.clone(),
                            claim.polarity,
                            claim.modality,
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }
        assert!(recovered_frames
            .windows(2)
            .all(|frames| frames[0] == frames[1]));
    }

    #[test]
    fn live_discourse_focus_projects_event_topic_by_node_identity_only() {
        let base = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                transfer_claims(),
                ApprovedVerbosityIR::Short,
            ),
            "transfer",
            ApprovedEventRealizationClassIR::Transfer,
            Some(ApprovedEventPredicateSenseIR::Transfer),
            None,
        );
        let context = ApprovedEventPragmaticContextIR {
            schema: APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA.into(),
            subject_node_id: "transfer".into(),
            requested_focus_node_id: None,
            backgrounded_node_ids: Vec::new(),
            recoverable_node_ids: Vec::new(),
            expression_preference: ApprovedEventExpressionPreferenceIR::Auto,
        };
        let focus = |surface: &str| {
            let mut state = DiscourseFocusStateIR::default();
            state.apply_turn(
                1,
                &[DiscourseFocusCandidateIR::explicit_topic(
                    surface,
                    Some("server"),
                )],
            );
            state
        };

        let first_focus = focus("표면 표현은 역할 근거가 아님");
        let states = base
            .derive_event_discourse_states(&first_focus, 1, std::slice::from_ref(&context))
            .unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0].current_topic,
            Some(ApprovedEventInformationRoleIR::Destination)
        );
        assert_eq!(
            states[0].requested_focus,
            ApprovedEventInformationRoleIR::Event
        );
        let projected = base
            .apply_event_pragmatic_contexts(&first_focus, 1, std::slice::from_ref(&context))
            .unwrap();
        let output = realize_document_response(&projected, LanguageCodeIR::Korean).unwrap();
        assert!(output
            .markdown
            .contains("서버에는 민수가 보고서를 전송했습니다."));
        assert!(output.validate(&projected));

        let relabeled_focus = focus("민수라고 써도 identity는 server");
        let relabeled = base
            .apply_event_pragmatic_contexts(&relabeled_focus, 1, &[context])
            .unwrap();
        assert_eq!(
            projected.event_realizations[0].information_structure,
            relabeled.event_realizations[0].information_structure
        );
    }

    #[test]
    fn pragmatic_selector_chooses_concise_or_explicit_verified_realization() {
        let build = |verbosity| {
            with_semantic_event_realization(
                response(
                    ApprovedDiscourseRelationIR::Statement,
                    ApprovedOperationIR::Assert,
                    transfer_claims(),
                    verbosity,
                ),
                "transfer",
                ApprovedEventRealizationClassIR::Transfer,
                Some(ApprovedEventPredicateSenseIR::Transfer),
                None,
            )
        };
        let context = ApprovedEventPragmaticContextIR {
            schema: APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA.into(),
            subject_node_id: "transfer".into(),
            requested_focus_node_id: Some("server".into()),
            backgrounded_node_ids: vec!["minsu".into()],
            recoverable_node_ids: vec!["minsu".into()],
            expression_preference: ApprovedEventExpressionPreferenceIR::Auto,
        };
        let focus = DiscourseFocusStateIR::default();

        let short = build(ApprovedVerbosityIR::Short)
            .apply_event_pragmatic_contexts(&focus, 0, std::slice::from_ref(&context))
            .unwrap();
        let short_output = realize_document_response(&short, LanguageCodeIR::Korean).unwrap();
        assert!(short_output
            .markdown
            .contains("서버에는 보고서가 전송됐습니다."));
        assert_eq!(
            short.event_realizations[0]
                .information_structure
                .as_ref()
                .unwrap()
                .omitted_roles,
            vec![ApprovedRelationTypeIR::Agent]
        );

        let explanatory = build(ApprovedVerbosityIR::Explanatory)
            .apply_event_pragmatic_contexts(&focus, 0, &[context])
            .unwrap();
        let explanatory_output =
            realize_document_response(&explanatory, LanguageCodeIR::Korean).unwrap();
        assert!(explanatory_output
            .markdown
            .contains("서버에는 민수가 보고서를 전송했습니다."));
        assert_eq!(
            explanatory.event_realizations[0].voice,
            Some(ApprovedEventVoiceIR::Active)
        );
        assert!(explanatory.event_realizations[0]
            .information_structure
            .as_ref()
            .unwrap()
            .omitted_roles
            .is_empty());
        let semantic_frame = |output: &DocumentResponseOutputIR| {
            output
                .semantic_interpretation
                .claims
                .iter()
                .map(|claim| {
                    (
                        claim.proposition_id.clone(),
                        claim.subject.clone(),
                        claim.relation,
                        claim.value.clone(),
                        claim.polarity,
                        claim.modality,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            semantic_frame(&short_output),
            semantic_frame(&explanatory_output)
        );
        assert!(short_output.validate(&short));
        assert!(explanatory_output.validate(&explanatory));
    }

    #[test]
    fn automatic_information_selection_falls_back_without_lexical_guessing() {
        let base = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                transfer_claims(),
                ApprovedVerbosityIR::Short,
            ),
            "transfer",
            ApprovedEventRealizationClassIR::Transfer,
            Some(ApprovedEventPredicateSenseIR::Transfer),
            None,
        );
        let unsupported = ApprovedEventDiscourseStateIR {
            schema: APPROVED_EVENT_DISCOURSE_STATE_SCHEMA.into(),
            subject_node_id: "transfer".into(),
            current_topic: Some(ApprovedEventInformationRoleIR::Source),
            requested_focus: ApprovedEventInformationRoleIR::Source,
            backgrounded_roles: Vec::new(),
            recoverable_roles: Vec::new(),
        };
        let projected = base
            .apply_event_discourse_states(std::slice::from_ref(&unsupported))
            .unwrap();
        assert!(projected.event_realizations[0].voice.is_none());
        assert!(projected.event_realizations[0]
            .information_structure
            .is_none());
        let fallback = realize_document_response(&projected, LanguageCodeIR::Korean).unwrap();
        assert!(fallback.plan.event_predicates.is_empty());
        assert_eq!(
            fallback.semantic_interpretation.recovered_claim_ids,
            vec!["P_AGENT", "P_THEME", "P_DESTINATION"]
        );

        let duplicate = base
            .apply_event_discourse_states(&[unsupported.clone(), unsupported])
            .unwrap();
        assert!(duplicate.event_realizations[0]
            .information_structure
            .is_none());

        let mut relabeled = base.clone();
        for claim in &mut relabeled.claims {
            if let ApprovedOpenValueIR::Lexical(node) = &mut claim.value {
                node.canonical_lexical_label = format!("새표현_{}", node.node_id);
            }
        }
        relabeled.semantic_sha256 = compositional_response_sha256(&relabeled);
        assert!(relabeled.validate());
        let typed_state = ApprovedEventDiscourseStateIR {
            schema: APPROVED_EVENT_DISCOURSE_STATE_SCHEMA.into(),
            subject_node_id: "transfer".into(),
            current_topic: None,
            requested_focus: ApprovedEventInformationRoleIR::Theme,
            backgrounded_roles: vec![ApprovedRelationTypeIR::Agent],
            recoverable_roles: vec![ApprovedRelationTypeIR::Agent],
        };
        let original_projection = base
            .apply_event_discourse_states(std::slice::from_ref(&typed_state))
            .unwrap();
        let relabeled_projection = relabeled
            .apply_event_discourse_states(std::slice::from_ref(&typed_state))
            .unwrap();
        assert_eq!(
            original_projection.event_realizations[0].voice,
            relabeled_projection.event_realizations[0].voice
        );
        assert_eq!(
            original_projection.event_realizations[0].information_structure,
            relabeled_projection.event_realizations[0].information_structure
        );

        let explicit = with_transfer_information_structure(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                transfer_claims(),
                ApprovedVerbosityIR::Short,
            ),
            ApprovedEventVoiceIR::Active,
            ApprovedEventInformationStructureIR {
                topic: None,
                focus: ApprovedEventInformationRoleIR::Agent,
                omitted_roles: Vec::new(),
                recoverable_roles: Vec::new(),
            },
        );
        assert_eq!(
            explicit
                .apply_event_discourse_states(&[typed_state])
                .unwrap(),
            explicit
        );
    }

    #[test]
    fn typed_information_structure_fails_closed_and_plan_tampering_is_rejected() {
        let base = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            transfer_claims(),
            ApprovedVerbosityIR::Short,
        );
        let information = ApprovedEventInformationStructureIR {
            topic: Some(ApprovedEventInformationRoleIR::Destination),
            focus: ApprovedEventInformationRoleIR::Theme,
            omitted_roles: vec![ApprovedRelationTypeIR::Agent],
            recoverable_roles: vec![ApprovedRelationTypeIR::Agent],
        };
        let approved = with_transfer_information_structure(
            base.clone(),
            ApprovedEventVoiceIR::Passive,
            information,
        );
        let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();

        let mut wrong_voice = output.plan.clone();
        wrong_voice.event_predicates[0].voice = Some(ApprovedEventVoiceIR::Active);
        wrong_voice.plan_sha256 = document_response_plan_sha256(&wrong_voice);
        assert!(!wrong_voice.validate(&approved));

        let mut wrong_topic = output.plan.clone();
        wrong_topic.event_predicates[0]
            .information_structure
            .as_mut()
            .unwrap()
            .topic = Some(ApprovedEventInformationRoleIR::Theme);
        wrong_topic.plan_sha256 = document_response_plan_sha256(&wrong_topic);
        assert!(!wrong_topic.validate(&approved));

        let mut wrong_focus = output.plan.clone();
        wrong_focus.event_predicates[0]
            .information_structure
            .as_mut()
            .unwrap()
            .focus = ApprovedEventInformationRoleIR::Event;
        wrong_focus.plan_sha256 = document_response_plan_sha256(&wrong_focus);
        assert!(!wrong_focus.validate(&approved));

        let mut wrong_omission = output.plan.clone();
        wrong_omission.event_predicates[0]
            .information_structure
            .as_mut()
            .unwrap()
            .omitted_roles
            .clear();
        wrong_omission.plan_sha256 = document_response_plan_sha256(&wrong_omission);
        assert!(!wrong_omission.validate(&approved));

        let mut wrong_recovery = output.plan.clone();
        wrong_recovery.event_predicates[0]
            .information_structure
            .as_mut()
            .unwrap()
            .recoverable_roles
            .clear();
        wrong_recovery.plan_sha256 = document_response_plan_sha256(&wrong_recovery);
        assert!(!wrong_recovery.validate(&approved));

        let valid_active = ApprovedEventInformationStructureIR {
            topic: None,
            focus: ApprovedEventInformationRoleIR::Agent,
            omitted_roles: Vec::new(),
            recoverable_roles: Vec::new(),
        };
        let mut both_contracts = with_transfer_perspective(
            base.clone(),
            Some(ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            }),
        );
        both_contracts.event_realizations[0].voice = Some(ApprovedEventVoiceIR::Active);
        both_contracts.event_realizations[0].information_structure = Some(valid_active.clone());
        both_contracts.semantic_sha256 = compositional_response_sha256(&both_contracts);
        assert!(!both_contracts.validate());

        let mut missing_voice = approved.clone();
        missing_voice.event_realizations[0].voice = None;
        missing_voice.semantic_sha256 = compositional_response_sha256(&missing_voice);
        assert!(!missing_voice.validate());

        let mut nonrecoverable_omission = approved.clone();
        nonrecoverable_omission.event_realizations[0]
            .information_structure
            .as_mut()
            .unwrap()
            .recoverable_roles
            .clear();
        nonrecoverable_omission.semantic_sha256 =
            compositional_response_sha256(&nonrecoverable_omission);
        assert!(!nonrecoverable_omission.validate());

        let mut active_omission = approved.clone();
        active_omission.event_realizations[0].voice = Some(ApprovedEventVoiceIR::Active);
        active_omission.event_realizations[0]
            .information_structure
            .as_mut()
            .unwrap()
            .topic = None;
        active_omission.event_realizations[0]
            .information_structure
            .as_mut()
            .unwrap()
            .focus = ApprovedEventInformationRoleIR::Agent;
        active_omission.semantic_sha256 = compositional_response_sha256(&active_omission);
        assert!(!active_omission.validate());

        let mut missing_topic_role =
            with_transfer_information_structure(base, ApprovedEventVoiceIR::Active, valid_active);
        missing_topic_role
            .claims
            .retain(|claim| claim.relation != ApprovedRelationTypeIR::Destination);
        missing_topic_role
            .clause_plan
            .retain(|unit| !unit.proposition_ids.iter().any(|id| id == "P_DESTINATION"));
        missing_topic_role.event_realizations[0]
            .information_structure
            .as_mut()
            .unwrap()
            .topic = Some(ApprovedEventInformationRoleIR::Destination);
        missing_topic_role.semantic_sha256 = compositional_response_sha256(&missing_topic_role);
        assert!(!missing_topic_role.validate());
    }

    #[test]
    fn transfer_requires_approved_perspective_and_typed_roles() {
        let base = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            transfer_claims(),
            ApprovedVerbosityIR::Short,
        );
        let absent_perspective = with_transfer_perspective(base.clone(), None);
        let fallback =
            realize_document_response(&absent_perspective, LanguageCodeIR::Korean).unwrap();
        assert!(fallback.validate(&absent_perspective));
        assert!(fallback.plan.event_predicates.is_empty());
        assert!(!fallback.markdown.contains("전송했습니다"));
        assert_eq!(
            fallback.semantic_interpretation.recovered_claim_ids,
            vec!["P_AGENT", "P_THEME", "P_DESTINATION"]
        );

        let perspective = Some(ApprovedEventPerspectiveIR {
            voice: ApprovedEventVoiceIR::Active,
            focus: ApprovedEventFocusIR::Agent,
        });
        let mut wrong_agent = base.clone();
        wrong_agent.claims[0].value = ApprovedOpenValueIR::Text("민수".into());
        wrong_agent.semantic_sha256 = compositional_response_sha256(&wrong_agent);
        let wrong_agent = with_transfer_perspective(wrong_agent, perspective);
        let agent_fallback =
            realize_document_response(&wrong_agent, LanguageCodeIR::Korean).unwrap();
        assert!(agent_fallback.plan.event_predicates.is_empty());

        let mut wrong_destination = base.clone();
        wrong_destination.claims[2].value = ApprovedOpenValueIR::Text("서버".into());
        wrong_destination.semantic_sha256 = compositional_response_sha256(&wrong_destination);
        let wrong_destination = with_transfer_perspective(wrong_destination, perspective);
        let destination_fallback =
            realize_document_response(&wrong_destination, LanguageCodeIR::Korean).unwrap();
        assert!(destination_fallback.plan.event_predicates.is_empty());

        let mut invalid_perspective = with_transfer_perspective(base, perspective);
        invalid_perspective.event_realizations[0].perspective = Some(ApprovedEventPerspectiveIR {
            voice: ApprovedEventVoiceIR::Passive,
            focus: ApprovedEventFocusIR::Destination,
        });
        invalid_perspective.semantic_sha256 = compositional_response_sha256(&invalid_perspective);
        assert!(!invalid_perspective.validate());
    }

    #[test]
    fn transfer_phase_and_optional_roles_remain_independent_typed_arguments() {
        let base = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            transfer_claims(),
            ApprovedVerbosityIR::Short,
        );
        let perspective = Some(ApprovedEventPerspectiveIR {
            voice: ApprovedEventVoiceIR::Active,
            focus: ApprovedEventFocusIR::Agent,
        });
        let cases = [
            (
                ApprovedEventPhaseIR::Planned,
                "전송할 예정입니다.",
                "will transfer 보고서 to 서버.",
            ),
            (
                ApprovedEventPhaseIR::Scheduled,
                "전송합니다.",
                "transfers 보고서 to 서버.",
            ),
            (
                ApprovedEventPhaseIR::Ongoing,
                "전송 중입니다.",
                "is transferring 보고서 to 서버.",
            ),
            (
                ApprovedEventPhaseIR::Completed,
                "전송했습니다.",
                "transferred 보고서 to 서버.",
            ),
            (
                ApprovedEventPhaseIR::Cancelled,
                "전송할 예정이었지만 취소됐습니다.",
                "was scheduled to transfer 보고서 to 서버 but the transfer was cancelled.",
            ),
        ];
        for (phase, ending, english_ending) in cases {
            let mut approved = with_transfer_perspective(base.clone(), perspective);
            approved.event_realizations[0].phase = Some(phase);
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            assert!(approved.validate());
            let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
            assert!(output.validate(&approved));
            assert!(output.markdown.contains(ending));
            let english = realize_document_response(&approved, LanguageCodeIR::English).unwrap();
            assert!(english.validate(&approved));
            assert!(english.markdown.contains(english_ending));
        }

        let mut claims = transfer_claims();
        claims.insert(
            1,
            claim(
                "P_PATIENT",
                "transfer",
                "전송",
                ApprovedRelationTypeIR::Patient,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "jisu".into(),
                    semantic_type: ApprovedSemanticTypeIR::Person,
                    canonical_lexical_label: "지수".into(),
                }),
            ),
        );
        claims.insert(
            3,
            claim(
                "P_SOURCE",
                "transfer",
                "전송",
                ApprovedRelationTypeIR::Source,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "archive".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "보관함".into(),
                }),
            ),
        );
        claims.extend([
            claim(
                "P_DATE",
                "transfer",
                "전송",
                ApprovedRelationTypeIR::Date,
                ApprovedOpenValueIR::Date {
                    year: 2026,
                    month: 10,
                    day: 12,
                },
            ),
            claim(
                "P_TIME",
                "transfer",
                "전송",
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 30,
                },
            ),
            claim(
                "P_LOCATION",
                "transfer",
                "전송",
                ApprovedRelationTypeIR::Location,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "workroom".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "작업실".into(),
                }),
            ),
            claim(
                "P_INSTRUMENT",
                "transfer",
                "전송",
                ApprovedRelationTypeIR::Instrument,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "usb".into(),
                    semantic_type: ApprovedSemanticTypeIR::Concept,
                    canonical_lexical_label: "USB".into(),
                }),
            ),
            claim(
                "P_MANNER",
                "transfer",
                "전송",
                ApprovedRelationTypeIR::Manner,
                ApprovedOpenValueIR::Text("암호화".into()),
            ),
        ]);
        let approved = with_transfer_perspective(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                claims,
                ApprovedVerbosityIR::Short,
            ),
            perspective,
        );
        let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
        assert!(output.validate(&approved));
        assert!(output.markdown.contains(
            "민수가 2026년 10월 12일 오후 4시 30분에 작업실의 보관함에서 지수에게 보고서를 USB를 이용해 암호화 방식으로 서버로 전송했습니다."
        ));
        assert_eq!(output.semantic_interpretation.recovered_claim_ids.len(), 10);
    }

    #[test]
    fn semantic_event_registry_reuses_one_frame_grammar_across_predicate_senses() {
        let perspective = Some(ApprovedEventPerspectiveIR {
            voice: ApprovedEventVoiceIR::Active,
            focus: ApprovedEventFocusIR::Agent,
        });
        let cases = [
            (
                "motion",
                ApprovedEventRealizationClassIR::Motion,
                ApprovedEventPredicateSenseIR::Move,
                vec![
                    claim(
                        "P_MOVE_AGENT",
                        "motion",
                        "이동",
                        ApprovedRelationTypeIR::Agent,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "minsu".into(),
                            semantic_type: ApprovedSemanticTypeIR::Person,
                            canonical_lexical_label: "민수".into(),
                        }),
                    ),
                    claim(
                        "P_MOVE_THEME",
                        "motion",
                        "이동",
                        ApprovedRelationTypeIR::Theme,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "box".into(),
                            semantic_type: ApprovedSemanticTypeIR::Concept,
                            canonical_lexical_label: "상자".into(),
                        }),
                    ),
                    claim(
                        "P_MOVE_SOURCE",
                        "motion",
                        "이동",
                        ApprovedRelationTypeIR::Source,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "warehouse".into(),
                            semantic_type: ApprovedSemanticTypeIR::Location,
                            canonical_lexical_label: "창고".into(),
                        }),
                    ),
                    claim(
                        "P_MOVE_DESTINATION",
                        "motion",
                        "이동",
                        ApprovedRelationTypeIR::Destination,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "workroom".into(),
                            semantic_type: ApprovedSemanticTypeIR::Location,
                            canonical_lexical_label: "작업실".into(),
                        }),
                    ),
                ],
                "민수가 창고에서 상자를 작업실로 옮겼습니다.",
                "민수 moved 상자 from 창고 to 작업실.",
            ),
            (
                "delivery",
                ApprovedEventRealizationClassIR::Transfer,
                ApprovedEventPredicateSenseIR::Deliver,
                vec![
                    claim(
                        "P_DELIVER_AGENT",
                        "delivery",
                        "전달",
                        ApprovedRelationTypeIR::Agent,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "minsu".into(),
                            semantic_type: ApprovedSemanticTypeIR::Person,
                            canonical_lexical_label: "민수".into(),
                        }),
                    ),
                    claim(
                        "P_DELIVER_PATIENT",
                        "delivery",
                        "전달",
                        ApprovedRelationTypeIR::Patient,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "jisu".into(),
                            semantic_type: ApprovedSemanticTypeIR::Person,
                            canonical_lexical_label: "지수".into(),
                        }),
                    ),
                    claim(
                        "P_DELIVER_THEME",
                        "delivery",
                        "전달",
                        ApprovedRelationTypeIR::Theme,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "report".into(),
                            semantic_type: ApprovedSemanticTypeIR::Concept,
                            canonical_lexical_label: "보고서".into(),
                        }),
                    ),
                ],
                "민수가 지수에게 보고서를 전달했습니다.",
                "민수 delivered 보고서 to 지수.",
            ),
            (
                "creation",
                ApprovedEventRealizationClassIR::Creation,
                ApprovedEventPredicateSenseIR::Create,
                vec![
                    claim(
                        "P_CREATE_AGENT",
                        "creation",
                        "생성",
                        ApprovedRelationTypeIR::Agent,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "minsu".into(),
                            semantic_type: ApprovedSemanticTypeIR::Person,
                            canonical_lexical_label: "민수".into(),
                        }),
                    ),
                    claim(
                        "P_CREATE_THEME",
                        "creation",
                        "생성",
                        ApprovedRelationTypeIR::Theme,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "report".into(),
                            semantic_type: ApprovedSemanticTypeIR::Concept,
                            canonical_lexical_label: "보고서".into(),
                        }),
                    ),
                ],
                "민수가 보고서를 생성했습니다.",
                "민수 created 보고서.",
            ),
            (
                "writing",
                ApprovedEventRealizationClassIR::Creation,
                ApprovedEventPredicateSenseIR::Write,
                vec![
                    claim(
                        "P_WRITE_AGENT",
                        "writing",
                        "작성",
                        ApprovedRelationTypeIR::Agent,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "minsu".into(),
                            semantic_type: ApprovedSemanticTypeIR::Person,
                            canonical_lexical_label: "민수".into(),
                        }),
                    ),
                    claim(
                        "P_WRITE_THEME",
                        "writing",
                        "작성",
                        ApprovedRelationTypeIR::Theme,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "notice".into(),
                            semantic_type: ApprovedSemanticTypeIR::Concept,
                            canonical_lexical_label: "안내문".into(),
                        }),
                    ),
                ],
                "민수가 안내문을 작성했습니다.",
                "민수 wrote 안내문.",
            ),
            (
                "inspection",
                ApprovedEventRealizationClassIR::Inspection,
                ApprovedEventPredicateSenseIR::Inspect,
                vec![
                    claim(
                        "P_INSPECT_AGENT",
                        "inspection",
                        "점검",
                        ApprovedRelationTypeIR::Agent,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "minsu".into(),
                            semantic_type: ApprovedSemanticTypeIR::Person,
                            canonical_lexical_label: "민수".into(),
                        }),
                    ),
                    claim(
                        "P_INSPECT_THEME",
                        "inspection",
                        "점검",
                        ApprovedRelationTypeIR::Theme,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "sensor".into(),
                            semantic_type: ApprovedSemanticTypeIR::Concept,
                            canonical_lexical_label: "안전 센서".into(),
                        }),
                    ),
                ],
                "민수가 안전 센서를 점검했습니다.",
                "민수 inspected 안전 센서.",
            ),
            (
                "change",
                ApprovedEventRealizationClassIR::StateChange,
                ApprovedEventPredicateSenseIR::Change,
                vec![
                    claim(
                        "P_CHANGE_AGENT",
                        "change",
                        "변경",
                        ApprovedRelationTypeIR::Agent,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "administrator".into(),
                            semantic_type: ApprovedSemanticTypeIR::Person,
                            canonical_lexical_label: "관리자".into(),
                        }),
                    ),
                    claim(
                        "P_CHANGE_THEME",
                        "change",
                        "변경",
                        ApprovedRelationTypeIR::Theme,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "status".into(),
                            semantic_type: ApprovedSemanticTypeIR::Concept,
                            canonical_lexical_label: "상태".into(),
                        }),
                    ),
                    claim(
                        "P_CHANGE_INITIAL",
                        "change",
                        "변경",
                        ApprovedRelationTypeIR::InitialState,
                        ApprovedOpenValueIR::Text("대기".into()),
                    ),
                    claim(
                        "P_CHANGE_RESULT",
                        "change",
                        "변경",
                        ApprovedRelationTypeIR::ResultState,
                        ApprovedOpenValueIR::Text("완료".into()),
                    ),
                ],
                "관리자가 상태를 “대기”에서 “완료”로 변경했습니다.",
                "관리자 changed 상태 from “대기” to “완료”.",
            ),
        ];

        for (subject_node_id, class, sense, claims, korean_surface, english_surface) in cases {
            let expected_claim_ids = claims
                .iter()
                .map(|claim| claim.proposition_id.clone())
                .collect::<Vec<_>>();
            let approved = with_semantic_event_realization(
                response(
                    ApprovedDiscourseRelationIR::Statement,
                    ApprovedOperationIR::Assert,
                    claims,
                    ApprovedVerbosityIR::Short,
                ),
                subject_node_id,
                class,
                Some(sense),
                perspective,
            );
            let korean = realize_document_response(&approved, LanguageCodeIR::Korean)
                .unwrap_or_else(|error| panic!("{subject_node_id} Korean: {error}"));
            assert!(korean.validate(&approved));
            assert!(korean.markdown.contains(korean_surface));
            assert_eq!(
                korean.semantic_interpretation.recovered_claim_ids,
                expected_claim_ids
            );
            assert_eq!(korean.plan.event_predicates.len(), 1);
            assert_eq!(korean.plan.event_predicates[0].predicate_sense, Some(sense));

            let english = realize_document_response(&approved, LanguageCodeIR::English)
                .unwrap_or_else(|error| panic!("{subject_node_id} English: {error}"));
            assert!(english.validate(&approved));
            assert!(english.markdown.contains(english_surface));
            assert_eq!(
                english.semantic_interpretation.recovered_claim_ids,
                expected_claim_ids
            );
        }
    }

    #[test]
    fn semantic_event_registry_fails_closed_on_unapproved_sense_or_incomplete_valency() {
        let perspective = Some(ApprovedEventPerspectiveIR {
            voice: ApprovedEventVoiceIR::Active,
            focus: ApprovedEventFocusIR::Agent,
        });
        let motion_claims = vec![
            claim(
                "P_AGENT",
                "motion",
                "이동",
                ApprovedRelationTypeIR::Agent,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "minsu".into(),
                    semantic_type: ApprovedSemanticTypeIR::Person,
                    canonical_lexical_label: "민수".into(),
                }),
            ),
            claim(
                "P_THEME",
                "motion",
                "이동",
                ApprovedRelationTypeIR::Theme,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "box".into(),
                    semantic_type: ApprovedSemanticTypeIR::Concept,
                    canonical_lexical_label: "상자".into(),
                }),
            ),
            claim(
                "P_DESTINATION",
                "motion",
                "이동",
                ApprovedRelationTypeIR::Destination,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "workroom".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "작업실".into(),
                }),
            ),
        ];
        let base = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            motion_claims,
            ApprovedVerbosityIR::Short,
        );
        let approved = with_semantic_event_realization(
            base.clone(),
            "motion",
            ApprovedEventRealizationClassIR::Motion,
            Some(ApprovedEventPredicateSenseIR::Move),
            perspective,
        );
        let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
        let mut tampered_plan = output.plan.clone();
        tampered_plan.event_predicates[0].predicate_sense =
            Some(ApprovedEventPredicateSenseIR::Create);
        tampered_plan.plan_sha256 = document_response_plan_sha256(&tampered_plan);
        assert!(!tampered_plan.validate(&approved));
        let mut tampered_role = output.plan.clone();
        tampered_role.event_predicates[0].arguments[0].role = DocumentEventArgumentRoleIR::Location;
        tampered_role.plan_sha256 = document_response_plan_sha256(&tampered_role);
        assert!(!tampered_role.validate(&approved));
        let mut tampered_phase = output.plan.clone();
        tampered_phase.event_predicates[0].phase = ApprovedEventPhaseIR::Ongoing;
        tampered_phase.plan_sha256 = document_response_plan_sha256(&tampered_phase);
        assert!(!tampered_phase.validate(&approved));
        let mut tampered_class = output.plan.clone();
        tampered_class.event_predicates[0].kind = DocumentEventPredicateKindIR::Creation;
        tampered_class.plan_sha256 = document_response_plan_sha256(&tampered_class);
        assert!(!tampered_class.validate(&approved));

        let mut wrong_class_sense = approved.clone();
        wrong_class_sense.event_realizations[0].predicate_sense =
            Some(ApprovedEventPredicateSenseIR::Create);
        wrong_class_sense.semantic_sha256 = compositional_response_sha256(&wrong_class_sense);
        assert!(!wrong_class_sense.validate());

        let missing_sense = with_semantic_event_realization(
            base,
            "motion",
            ApprovedEventRealizationClassIR::Motion,
            None,
            perspective,
        );
        let fallback = realize_document_response(&missing_sense, LanguageCodeIR::Korean).unwrap();
        assert!(fallback.validate(&missing_sense));
        assert!(fallback.plan.event_predicates.is_empty());
        assert!(!fallback.markdown.contains("이동했습니다"));
        assert_eq!(
            fallback.semantic_interpretation.recovered_claim_ids,
            vec!["P_AGENT", "P_THEME", "P_DESTINATION"]
        );

        let mut extra_role_claims = approved.claims.clone();
        extra_role_claims.push(claim(
            "P_CAPACITY",
            "motion",
            "이동",
            ApprovedRelationTypeIR::Capacity,
            ApprovedOpenValueIR::Integer(3),
        ));
        let extra_role = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                extra_role_claims,
                ApprovedVerbosityIR::Short,
            ),
            "motion",
            ApprovedEventRealizationClassIR::Motion,
            Some(ApprovedEventPredicateSenseIR::Move),
            perspective,
        );
        let fallback = realize_document_response(&extra_role, LanguageCodeIR::Korean).unwrap();
        assert!(fallback.validate(&extra_role));
        assert!(fallback.plan.event_predicates.is_empty());
        assert!(!fallback.markdown.contains("이동했습니다"));
        assert_eq!(
            fallback.semantic_interpretation.recovered_claim_ids.len(),
            4
        );

        let incomplete_delivery = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                transfer_claims(),
                ApprovedVerbosityIR::Short,
            ),
            "transfer",
            ApprovedEventRealizationClassIR::Transfer,
            Some(ApprovedEventPredicateSenseIR::Deliver),
            perspective,
        );
        let fallback =
            realize_document_response(&incomplete_delivery, LanguageCodeIR::Korean).unwrap();
        assert!(fallback.validate(&incomplete_delivery));
        assert!(fallback.plan.event_predicates.is_empty());
        assert!(!fallback.markdown.contains("전달했습니다"));
        assert_eq!(
            fallback.semantic_interpretation.recovered_claim_ids,
            vec!["P_AGENT", "P_THEME", "P_DESTINATION"]
        );
    }

    #[test]
    fn coating_and_attachment_are_distinct_typed_predicate_senses() {
        let perspective = Some(ApprovedEventPerspectiveIR {
            voice: ApprovedEventVoiceIR::Active,
            focus: ApprovedEventFocusIR::Agent,
        });
        let cases = [
            (
                "coating",
                ApprovedEventRealizationClassIR::Transfer,
                ApprovedEventPredicateSenseIR::Apply,
                "보존처리사가 강화제를 세필로 균열 가장자리에 도포했습니다.",
            ),
            (
                "attachment",
                ApprovedEventRealizationClassIR::StateChange,
                ApprovedEventPredicateSenseIR::Attach,
                "보존처리사가 등록표를 세필로 표본에 부착했습니다.",
            ),
        ];
        for (subject_node_id, class, sense, expected) in cases {
            let claims = vec![
                claim(
                    "P_AGENT",
                    subject_node_id,
                    "처리",
                    ApprovedRelationTypeIR::Agent,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "conservator".into(),
                        semantic_type: ApprovedSemanticTypeIR::Person,
                        canonical_lexical_label: "보존처리사".into(),
                    }),
                ),
                claim(
                    "P_THEME",
                    subject_node_id,
                    "처리",
                    ApprovedRelationTypeIR::Theme,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "material".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: if sense == ApprovedEventPredicateSenseIR::Apply {
                            "강화제".into()
                        } else {
                            "등록표".into()
                        },
                    }),
                ),
                claim(
                    "P_TARGET",
                    subject_node_id,
                    "처리",
                    ApprovedRelationTypeIR::Target,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "target".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: if sense == ApprovedEventPredicateSenseIR::Apply {
                            "균열 가장자리".into()
                        } else {
                            "표본".into()
                        },
                    }),
                ),
                claim(
                    "P_INSTRUMENT",
                    subject_node_id,
                    "처리",
                    ApprovedRelationTypeIR::Instrument,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "fine_brush".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: "세필".into(),
                    }),
                ),
            ];
            let approved = with_semantic_event_realization(
                response(
                    ApprovedDiscourseRelationIR::Statement,
                    ApprovedOperationIR::Assert,
                    claims,
                    ApprovedVerbosityIR::Short,
                ),
                subject_node_id,
                class,
                Some(sense),
                perspective,
            );
            let output = realize_document_response(&approved, LanguageCodeIR::Korean)
                .unwrap_or_else(|error| panic!("{subject_node_id}: {error}"));
            assert_eq!(output.markdown, expected);
            assert!(output.validate(&approved));
        }
    }

    #[test]
    fn semantic_event_registry_contract_is_closed_order_independent_and_non_authoritative() {
        let specs = crate::approved_response::APPROVED_EVENT_PREDICATE_SPECS;
        assert!(!specs.is_empty());
        assert_eq!(EVENT_LANGUAGE_CODECS.len(), specs.len());

        let mut senses = Vec::new();
        for spec in specs.iter().rev().copied() {
            assert_eq!(
                approved_event_predicate_spec(spec.event_class, spec.predicate_sense),
                Some(spec)
            );
            assert!(!spec.required_roles.is_empty());
            assert!(!spec.allowed_phases.is_empty());
            assert!(spec
                .required_roles
                .iter()
                .all(|role| !spec.optional_roles.contains(role)));
            assert!(senses.iter().all(|sense| *sense != spec.predicate_sense));
            senses.push(spec.predicate_sense);

            let codec = EVENT_LANGUAGE_CODECS
                .iter()
                .rev()
                .copied()
                .find(|codec| codec.predicate_sense == spec.predicate_sense)
                .expect("each semantic specification has a codec");
            assert_eq!(event_language_codec(spec.predicate_sense), codec);
        }

        let approved = with_transfer_perspective(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                transfer_claims(),
                ApprovedVerbosityIR::Short,
            ),
            Some(ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            }),
        );
        let semantic_hash = approved.semantic_sha256.clone();
        let serialized = serde_json::to_string(&approved).unwrap();
        let _codec = event_language_codec(ApprovedEventPredicateSenseIR::Transfer);
        assert_eq!(approved.semantic_sha256, semantic_hash);
        assert!(!serialized.contains("transferred"));
        assert!(!serialized.contains("전송했습니다"));
    }

    #[test]
    fn every_registered_event_predicate_executes_with_its_minimal_typed_valency_frame() {
        // This is deliberately registry-driven: adding a predicate sense is
        // not complete until its declared event class and required roles can
        // travel through the shared realizer and the semantic inverse.
        for (index, spec) in crate::approved_response::APPROVED_EVENT_PREDICATE_SPECS
            .iter()
            .copied()
            .enumerate()
        {
            let event_id = format!("registered_event_{index}");
            let claims = spec
                .required_roles
                .iter()
                .enumerate()
                .map(|(role_index, role)| {
                    let (node_id, semantic_type, label) = match role {
                        ApprovedRelationTypeIR::Agent => {
                            ("operator", ApprovedSemanticTypeIR::Person, "운영자")
                        }
                        ApprovedRelationTypeIR::Patient => {
                            ("recipient", ApprovedSemanticTypeIR::Person, "담당자")
                        }
                        ApprovedRelationTypeIR::Source => {
                            ("source", ApprovedSemanticTypeIR::Location, "보관 구역")
                        }
                        ApprovedRelationTypeIR::Destination => {
                            ("destination", ApprovedSemanticTypeIR::Location, "작업 구역")
                        }
                        ApprovedRelationTypeIR::Target => {
                            ("target", ApprovedSemanticTypeIR::Concept, "대상 설비")
                        }
                        ApprovedRelationTypeIR::Theme => {
                            ("theme", ApprovedSemanticTypeIR::Concept, "장비")
                        }
                        ApprovedRelationTypeIR::InitialState => {
                            ("initial_state", ApprovedSemanticTypeIR::State, "초기 상태")
                        }
                        ApprovedRelationTypeIR::ResultState => {
                            ("result_state", ApprovedSemanticTypeIR::State, "변경 상태")
                        }
                        unexpected => panic!(
                            "registered event predicate contains unsupported required role: {unexpected:?}"
                        ),
                    };
                    claim(
                        &format!("P_REGISTERED_{index}_{role_index}"),
                        &event_id,
                        "작업",
                        *role,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: format!("{node_id}_{index}_{role_index}"),
                            semantic_type,
                            canonical_lexical_label: label.into(),
                        }),
                    )
                })
                .collect();
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                claims,
                ApprovedVerbosityIR::Short,
            );
            approved
                .event_realizations
                .push(crate::approved_response::ApprovedEventRealizationIR {
                    subject_node_id: event_id.clone(),
                    class: spec.event_class,
                    predicate_sense: Some(spec.predicate_sense),
                    phase: Some(ApprovedEventPhaseIR::Completed),
                    perspective: None,
                    voice: None,
                    information_structure: None,
                });
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            assert!(
                approved.validate(),
                "registered predicate frame is invalid at index {index}: {:?}",
                spec.predicate_sense
            );
            let output = realize_document_response(&approved, LanguageCodeIR::Korean)
                .unwrap_or_else(|error| panic!("{:?}: {error}", spec.predicate_sense));
            assert!(
                output.validate(&approved),
                "{:?}: {}",
                spec.predicate_sense,
                output.markdown
            );
            assert_eq!(output.unsupported_claims, 0, "{:?}", spec.predicate_sense);
        }
    }

    #[test]
    fn semantic_event_realization_is_independent_of_canonical_role_order() {
        let mut shuffled = transfer_claims();
        shuffled.rotate_left(2);
        let approved = with_transfer_perspective(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                shuffled,
                ApprovedVerbosityIR::Short,
            ),
            Some(ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            }),
        );

        let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
        assert_eq!(output.markdown, "민수가 보고서를 서버로 전송했습니다.");
        assert!(output.validate(&approved));
        assert_eq!(output.unsupported_claims, 0);
    }

    #[test]
    fn expanded_predicate_registry_reuses_roles_voice_and_semantic_inverse() {
        let active_agent = Some(ApprovedEventPerspectiveIR {
            voice: ApprovedEventVoiceIR::Active,
            focus: ApprovedEventFocusIR::Agent,
        });
        let send = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                transfer_claims(),
                ApprovedVerbosityIR::Short,
            ),
            "transfer",
            ApprovedEventRealizationClassIR::Transfer,
            Some(ApprovedEventPredicateSenseIR::Send),
            active_agent,
        );
        let send_output = realize_document_response(&send, LanguageCodeIR::Korean).unwrap();
        assert!(send_output
            .markdown
            .contains("민수가 보고서를 서버로 발송했습니다."));
        assert!(send_output.validate(&send));

        let mut give_claims = transfer_claims();
        give_claims[2].relation = ApprovedRelationTypeIR::Patient;
        if let ApprovedOpenValueIR::Lexical(node) = &mut give_claims[2].value {
            node.node_id = "customer".into();
            node.semantic_type = ApprovedSemanticTypeIR::Person;
            node.canonical_lexical_label = "고객".into();
        }
        give_claims.swap(1, 2);
        let give = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                give_claims,
                ApprovedVerbosityIR::Short,
            ),
            "transfer",
            ApprovedEventRealizationClassIR::Transfer,
            Some(ApprovedEventPredicateSenseIR::Give),
            active_agent,
        );
        let give_output = realize_document_response(&give, LanguageCodeIR::Korean).unwrap();
        assert!(
            give_output
                .markdown
                .contains("민수가 고객에게 보고서를 제공했습니다."),
            "{}",
            give_output.markdown
        );
        assert!(give_output.validate(&give));

        let state_claims = || {
            vec![
                claim(
                    "P_AGENT",
                    "state_event",
                    "상태 변화",
                    ApprovedRelationTypeIR::Agent,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "manager".into(),
                        semantic_type: ApprovedSemanticTypeIR::Person,
                        canonical_lexical_label: "관리자".into(),
                    }),
                ),
                claim(
                    "P_THEME",
                    "state_event",
                    "상태 변화",
                    ApprovedRelationTypeIR::Theme,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "service".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: "서비스".into(),
                    }),
                ),
            ]
        };
        for (sense, surface) in [
            (ApprovedEventPredicateSenseIR::Start, "시작했습니다"),
            (ApprovedEventPredicateSenseIR::Complete, "완료했습니다"),
            (ApprovedEventPredicateSenseIR::Open, "개방했습니다"),
            (ApprovedEventPredicateSenseIR::Close, "폐쇄했습니다"),
            (ApprovedEventPredicateSenseIR::Reduce, "축소했습니다"),
            (ApprovedEventPredicateSenseIR::Remove, "제거했습니다"),
        ] {
            let approved = with_semantic_event_realization(
                response(
                    ApprovedDiscourseRelationIR::Statement,
                    ApprovedOperationIR::Assert,
                    state_claims(),
                    ApprovedVerbosityIR::Short,
                ),
                "state_event",
                ApprovedEventRealizationClassIR::StateChange,
                Some(sense),
                active_agent,
            );
            let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
            assert!(output.markdown.contains(surface), "{}", output.markdown);
            assert!(output.validate(&approved));
            assert_eq!(output.semantic_interpretation.recovered_claim_ids.len(), 2);
        }

        let mut post_claims = state_claims();
        if let ApprovedOpenValueIR::Lexical(theme) = &mut post_claims[1].value {
            theme.node_id = "work_hours_notice".into();
            theme.canonical_lexical_label = "작업 시간 공지".into();
        }
        post_claims.push(claim(
            "P_TARGET",
            "state_event",
            "상태 변화",
            ApprovedRelationTypeIR::Target,
            ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                node_id: "notice_board".into(),
                semantic_type: ApprovedSemanticTypeIR::Location,
                canonical_lexical_label: "게시판".into(),
            }),
        ));
        let post = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                post_claims,
                ApprovedVerbosityIR::Short,
            ),
            "state_event",
            ApprovedEventRealizationClassIR::StateChange,
            Some(ApprovedEventPredicateSenseIR::Post),
            active_agent,
        );
        let post_output = realize_document_response(&post, LanguageCodeIR::Korean).unwrap();
        assert!(
            post_output
                .markdown
                .contains("관리자가 작업 시간 공지를 게시판에 게시했습니다."),
            "{}",
            post_output.markdown
        );
        assert!(post_output.validate(&post));
        assert_eq!(post_output.semantic_interpretation.recovered_claim_ids.len(), 3);

        let mut supply_claims = state_claims();
        if let ApprovedOpenValueIR::Lexical(theme) = &mut supply_claims[1].value {
            theme.node_id = "clean_water".into();
            theme.canonical_lexical_label = "깨끗한 물".into();
        }
        supply_claims.push(claim(
            "P_TARGET",
            "state_event",
            "상태 변화",
            ApprovedRelationTypeIR::Target,
            ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                node_id: "shallow_vessel".into(),
                semantic_type: ApprovedSemanticTypeIR::Concept,
                canonical_lexical_label: "얕은 용기".into(),
            }),
        ));
        let supply = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                supply_claims,
                ApprovedVerbosityIR::Short,
            ),
            "state_event",
            ApprovedEventRealizationClassIR::StateChange,
            Some(ApprovedEventPredicateSenseIR::Supply),
            active_agent,
        );
        let supply_output = realize_document_response(&supply, LanguageCodeIR::Korean).unwrap();
        assert!(
            supply_output
                .markdown
                .contains("관리자가 깨끗한 물을 얕은 용기에 공급했습니다."),
            "{}",
            supply_output.markdown
        );
        assert!(supply_output.validate(&supply));
        assert_eq!(supply_output.semantic_interpretation.recovered_claim_ids.len(), 3);
    }

    #[test]
    fn ongoing_start_uses_a_finite_progressive_predicate() {
        let claims = vec![
            claim(
                "P_START_AGENT",
                "disinfection_start",
                "소독 시작",
                ApprovedRelationTypeIR::Agent,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "environment_team".into(),
                    semantic_type: ApprovedSemanticTypeIR::Person,
                    canonical_lexical_label: "환경관리팀".into(),
                }),
            ),
            claim(
                "P_START_THEME",
                "disinfection_start",
                "소독 시작",
                ApprovedRelationTypeIR::Theme,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "terminal_disinfection".into(),
                    semantic_type: ApprovedSemanticTypeIR::Concept,
                    canonical_lexical_label: "퇴실 병상 소독".into(),
                }),
            ),
        ];
        let mut approved = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                claims,
                ApprovedVerbosityIR::Short,
            ),
            "disinfection_start",
            ApprovedEventRealizationClassIR::StateChange,
            Some(ApprovedEventPredicateSenseIR::Start),
            Some(ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            }),
        );
        approved.event_realizations[0].phase = Some(ApprovedEventPhaseIR::Ongoing);
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let output = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("ongoing start should roundtrip");
        assert_eq!(
            output.markdown,
            "환경관리팀이 퇴실 병상 소독을 시작하고 있습니다."
        );
        assert!(output.validate(&approved));
    }

    #[test]
    fn fill_uses_target_as_the_affected_argument_and_theme_as_material() {
        let approved = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![
                    claim(
                        "P_FILL_THEME",
                        "surface_fill",
                        "균열 충전",
                        ApprovedRelationTypeIR::Theme,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "sealant".into(),
                            semantic_type: ApprovedSemanticTypeIR::Concept,
                            canonical_lexical_label: "보수재".into(),
                        }),
                    ),
                    claim(
                        "P_FILL_TARGET",
                        "surface_fill",
                        "균열 충전",
                        ApprovedRelationTypeIR::Target,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "wall_crack".into(),
                            semantic_type: ApprovedSemanticTypeIR::Concept,
                            canonical_lexical_label: "벽면 균열".into(),
                        }),
                    ),
                ],
                ApprovedVerbosityIR::Short,
            ),
            "surface_fill",
            ApprovedEventRealizationClassIR::StateChange,
            Some(ApprovedEventPredicateSenseIR::Fill),
            Some(ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Passive,
                focus: ApprovedEventFocusIR::Theme,
            }),
        );

        let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
        assert_eq!(output.markdown, "벽면 균열이 보수재로 메워졌습니다.");
        assert!(output.validate(&approved));
        assert_eq!(output.unsupported_claims, 0);
    }

    #[test]
    fn withdraw_keeps_the_moving_theme_as_an_active_korean_subject() {
        let mut approved = with_semantic_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_WITHDRAW_THEME",
                    "evacuate",
                    "대피",
                    ApprovedRelationTypeIR::Theme,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "worker".into(),
                        semantic_type: ApprovedSemanticTypeIR::Person,
                        canonical_lexical_label: "작업자".into(),
                    }),
                )],
                ApprovedVerbosityIR::Short,
            ),
            "evacuate",
            ApprovedEventRealizationClassIR::Motion,
            Some(ApprovedEventPredicateSenseIR::Withdraw),
            Some(ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Passive,
                focus: ApprovedEventFocusIR::Theme,
            }),
        );
        approved.event_realizations[0].phase = Some(ApprovedEventPhaseIR::Planned);
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        assert!(approved.validate());

        let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
        assert_eq!(output.markdown, "작업자는 후퇴할 예정입니다.");
        assert!(output.validate(&approved));
        assert_eq!(output.unsupported_claims, 0);
    }

    #[test]
    fn relational_target_predicates_use_counterpart_particles_in_korean() {
        for (subject_id, class, sense, theme, target, expected) in [
            (
                "usage_comparison",
                ApprovedEventRealizationClassIR::Inspection,
                ApprovedEventPredicateSenseIR::Compare,
                "통신사 A 사용량",
                "통신사 B 사용량",
                "통신사 A 사용량은 통신사 B 사용량과 대조됐습니다.",
            ),
            (
                "transport_connection",
                ApprovedEventRealizationClassIR::StateChange,
                ApprovedEventPredicateSenseIR::Connect,
                "대중교통 안내",
                "셔틀 정류장",
                "대중교통 안내는 셔틀 정류장과 연계됐습니다.",
            ),
        ] {
            let approved = with_semantic_event_realization(
                response(
                    ApprovedDiscourseRelationIR::Statement,
                    ApprovedOperationIR::Assert,
                    vec![
                        claim(
                            &format!("P_{subject_id}_THEME"),
                            subject_id,
                            "연계 또는 대조",
                            ApprovedRelationTypeIR::Theme,
                            ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                                node_id: format!("{subject_id}_theme"),
                                semantic_type: ApprovedSemanticTypeIR::Concept,
                                canonical_lexical_label: theme.into(),
                            }),
                        ),
                        claim(
                            &format!("P_{subject_id}_TARGET"),
                            subject_id,
                            "연계 또는 대조",
                            ApprovedRelationTypeIR::Target,
                            ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                                node_id: format!("{subject_id}_target"),
                                semantic_type: ApprovedSemanticTypeIR::Concept,
                                canonical_lexical_label: target.into(),
                            }),
                        ),
                    ],
                    ApprovedVerbosityIR::Short,
                ),
                subject_id,
                class,
                Some(sense),
                Some(ApprovedEventPerspectiveIR {
                    voice: ApprovedEventVoiceIR::Passive,
                    focus: ApprovedEventFocusIR::Theme,
                }),
            );
            let output = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
            assert_eq!(output.markdown, expected);
            assert!(output.validate(&approved));
            assert_eq!(output.unsupported_claims, 0);
        }
    }

    #[test]
    fn approved_event_class_and_phase_select_predicate_without_label_guessing() {
        let base = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![
                claim(
                    "P_TIME",
                    "operation",
                    "작업",
                    ApprovedRelationTypeIR::Time,
                    ApprovedOpenValueIR::Clock {
                        hour: 14,
                        minute: 0,
                    },
                ),
                claim(
                    "P_LOCATION",
                    "operation",
                    "작업",
                    ApprovedRelationTypeIR::Location,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "workroom".into(),
                        semantic_type: ApprovedSemanticTypeIR::Location,
                        canonical_lexical_label: "작업실".into(),
                    }),
                ),
            ],
            ApprovedVerbosityIR::Short,
        );

        let cases = [
            (
                ApprovedEventRealizationClassIR::HostedEvent,
                ApprovedEventPhaseIR::Scheduled,
                "작업은 오후 2시에 작업실에서 열립니다.",
                "작업 takes place at 14:00 in 작업실.",
            ),
            (
                ApprovedEventRealizationClassIR::ScheduledProcess,
                ApprovedEventPhaseIR::Ongoing,
                "작업은 오후 2시에 작업실에서 진행 중입니다.",
                "작업 is in progress at 14:00 in 작업실.",
            ),
            (
                ApprovedEventRealizationClassIR::Departure,
                ApprovedEventPhaseIR::Completed,
                "작업은 오후 2시에 작업실에서 출발했습니다.",
                "작업 departed from 작업실 at 14:00.",
            ),
            (
                ApprovedEventRealizationClassIR::Arrival,
                ApprovedEventPhaseIR::Planned,
                "작업은 오후 2시에 작업실에 도착할 예정입니다.",
                "작업 is planned to arrive at 작업실 at 14:00.",
            ),
        ];
        for (class, phase, korean_surface, english_surface) in cases {
            let approved = with_event_realization(base.clone(), "operation", class, phase);
            let korean = realize_document_response(&approved, LanguageCodeIR::Korean).unwrap();
            assert!(korean.validate(&approved));
            assert_eq!(korean.markdown, korean_surface);
            assert_eq!(
                korean.semantic_interpretation.recovered_claim_ids,
                vec!["P_TIME", "P_LOCATION"]
            );
            let english = realize_document_response(&approved, LanguageCodeIR::English).unwrap();
            assert!(english.validate(&approved));
            assert_eq!(english.markdown, english_surface);
        }

        let mut informal_arrival = with_event_realization(
            base.clone(),
            "operation",
            ApprovedEventRealizationClassIR::Arrival,
            ApprovedEventPhaseIR::Planned,
        );
        informal_arrival.style.register = LanguageRegisterIR::Informal;
        informal_arrival.semantic_sha256 = compositional_response_sha256(&informal_arrival);
        assert!(informal_arrival.validate());
        let informal =
            realize_document_response(&informal_arrival, LanguageCodeIR::Korean).unwrap();
        assert_eq!(
            informal.markdown,
            "작업은 오후 2시에 작업실에 도착할 예정이야."
        );
        assert!(informal.validate(&informal_arrival));

        let process = with_event_realization(
            base.clone(),
            "operation",
            ApprovedEventRealizationClassIR::ScheduledProcess,
            ApprovedEventPhaseIR::Ongoing,
        );
        let output = realize_document_response(&process, LanguageCodeIR::Korean).unwrap();
        let mut wrong_class = output.plan.clone();
        wrong_class.event_predicates[0].kind = DocumentEventPredicateKindIR::HostedOccurrence;
        wrong_class.plan_sha256 = document_response_plan_sha256(&wrong_class);
        assert!(!wrong_class.validate(&process));
        let mut wrong_phase = output.plan.clone();
        wrong_phase.event_predicates[0].phase = ApprovedEventPhaseIR::Completed;
        wrong_phase.plan_sha256 = document_response_plan_sha256(&wrong_phase);
        assert!(!wrong_phase.validate(&process));

        let mut phase_absent = base.clone();
        phase_absent.event_realizations =
            vec![crate::approved_response::ApprovedEventRealizationIR {
                subject_node_id: "operation".into(),
                class: ApprovedEventRealizationClassIR::ScheduledProcess,
                predicate_sense: None,
                phase: None,
                perspective: None,
                voice: None,
                information_structure: None,
            }];
        phase_absent.semantic_sha256 = compositional_response_sha256(&phase_absent);
        assert!(phase_absent.validate());
        let fallback = realize_document_response(&phase_absent, LanguageCodeIR::Korean).unwrap();
        assert!(fallback.plan.event_predicates.is_empty());
        assert_eq!(
            fallback.markdown,
            "작업 시간은 오후 2시이고, 장소는 작업실입니다."
        );

        let mut phase_conflict = base;
        phase_conflict.event_realizations =
            vec![crate::approved_response::ApprovedEventRealizationIR {
                subject_node_id: "operation".into(),
                class: ApprovedEventRealizationClassIR::Arrival,
                predicate_sense: None,
                phase: Some(ApprovedEventPhaseIR::Cancelled),
                perspective: None,
                voice: None,
                information_structure: None,
            }];
        phase_conflict.semantic_sha256 = compositional_response_sha256(&phase_conflict);
        assert!(!phase_conflict.validate());
    }

    #[test]
    fn event_predicate_requires_an_event_typed_subject_and_typed_location() {
        let mut person_claims = vec![
            claim(
                "P_DATE",
                "person",
                "민수",
                ApprovedRelationTypeIR::Date,
                ApprovedOpenValueIR::Date {
                    year: 2026,
                    month: 10,
                    day: 12,
                },
            ),
            claim(
                "P_TIME",
                "person",
                "민수",
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 30,
                },
            ),
        ];
        for claim in &mut person_claims {
            claim.subject.semantic_type = ApprovedSemanticTypeIR::Person;
        }
        let person = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            person_claims,
            ApprovedVerbosityIR::Short,
        );
        let person_output = realize_document_response(&person, LanguageCodeIR::Korean).unwrap();
        assert!(person_output.plan.event_predicates.is_empty());
        assert!(!person_output.markdown.contains("열립니다"));

        let absent_class = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![
                claim(
                    "P_TIME",
                    "event",
                    "행사",
                    ApprovedRelationTypeIR::Time,
                    ApprovedOpenValueIR::Clock {
                        hour: 16,
                        minute: 30,
                    },
                ),
                claim(
                    "P_LOCATION",
                    "event",
                    "행사",
                    ApprovedRelationTypeIR::Location,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "hall_b".into(),
                        semantic_type: ApprovedSemanticTypeIR::Location,
                        canonical_lexical_label: "B홀".into(),
                    }),
                ),
            ],
            ApprovedVerbosityIR::Short,
        );
        let absent_class_output =
            realize_document_response(&absent_class, LanguageCodeIR::Korean).unwrap();
        assert!(absent_class_output.plan.event_predicates.is_empty());
        assert!(absent_class_output
            .markdown
            .contains("행사 시간은 오후 4시 30분이고, 장소는 B홀입니다."));

        let untyped_location = with_event_realization(
            response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![
                    claim(
                        "P_TIME",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 16,
                            minute: 30,
                        },
                    ),
                    claim(
                        "P_LOCATION",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Location,
                        ApprovedOpenValueIR::Text("B홀".into()),
                    ),
                ],
                ApprovedVerbosityIR::Short,
            ),
            "event",
            ApprovedEventRealizationClassIR::HostedEvent,
            ApprovedEventPhaseIR::Scheduled,
        );
        let location_output =
            realize_document_response(&untyped_location, LanguageCodeIR::Korean).unwrap();
        assert!(location_output.plan.event_predicates.is_empty());
        assert!(!location_output.markdown.contains("열립니다"));
    }

    #[test]
    fn mixed_long_response_is_partitioned_into_ordered_semantic_sections() {
        let response = with_event_realization(
            response(
                ApprovedDiscourseRelationIR::Explanation,
                ApprovedOperationIR::Explain,
                vec![
                    claim(
                        "P_NAME",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Name,
                        ApprovedOpenValueIR::Text("가을 포럼".into()),
                    ),
                    claim(
                        "P_CONFIRMED",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Confirmed,
                        ApprovedOpenValueIR::Boolean(true),
                    ),
                    claim(
                        "P_DATE",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Date,
                        ApprovedOpenValueIR::Date {
                            year: 2026,
                            month: 10,
                            day: 12,
                        },
                    ),
                    claim(
                        "P_TIME",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Time,
                        ApprovedOpenValueIR::Clock {
                            hour: 16,
                            minute: 30,
                        },
                    ),
                    claim(
                        "P_LOCATION",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Location,
                        ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                            node_id: "hall_b".into(),
                            semantic_type: ApprovedSemanticTypeIR::Location,
                            canonical_lexical_label: "B홀".into(),
                        }),
                    ),
                    claim(
                        "P_ENTRY",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Entry,
                        ApprovedOpenValueIR::Boolean(true),
                    ),
                    claim(
                        "P_CAPACITY",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Capacity,
                        ApprovedOpenValueIR::Integer(120),
                    ),
                    claim(
                        "P_COUNT",
                        "event",
                        "행사",
                        ApprovedRelationTypeIR::Count,
                        ApprovedOpenValueIR::Integer(8),
                    ),
                ],
                ApprovedVerbosityIR::Explanatory,
            ),
            "event",
            ApprovedEventRealizationClassIR::HostedEvent,
            ApprovedEventPhaseIR::Scheduled,
        );
        let output = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(output.validate(&response));
        assert_eq!(
            output
                .plan
                .sections
                .iter()
                .map(|section| section.role)
                .collect::<Vec<_>>(),
            vec![
                DocumentResponseRoleIR::CoreContent,
                DocumentResponseRoleIR::Timeline,
                DocumentResponseRoleIR::PlaceAndAccess,
                DocumentResponseRoleIR::Measurement,
            ]
        );
        assert!(output.markdown.contains("## 이유와 설명"));
        for heading in [
            "### 1. 핵심 내용",
            "### 2. 시간 정보",
            "### 3. 장소와 이용",
            "### 4. 수량과 규모",
        ] {
            assert!(output.markdown.contains(heading), "{heading}");
        }
        for continuation in [
            "행사는 2026년 10월 12일 오후 4시 30분에 B홀에서 열립니다.",
            "행사는 입장할 수 있습니다.",
            "정리하면, 8개입니다.",
        ] {
            assert!(output.markdown.contains(continuation), "{continuation}");
        }
        assert_eq!(
            output.semantic_interpretation.recovered_claim_ids,
            response
                .claims
                .iter()
                .map(|claim| claim.proposition_id.clone())
                .collect::<Vec<_>>()
        );

        let mut reordered = output.plan.clone();
        reordered.covered_claim_ids.swap(0, 1);
        reordered.plan_sha256 = document_response_plan_sha256(&reordered);
        assert!(!reordered.validate(&response));

        let english = realize_document_response(&response, LanguageCodeIR::English).unwrap();
        assert!(english.validate(&response));
        for heading in [
            "## Reasons and explanation",
            "### 1. Key points",
            "### 2. Time information",
            "### 3. Location and access",
            "### 4. Quantities and scale",
        ] {
            assert!(english.markdown.contains(heading), "{heading}");
        }
        for continuation in [
            "행사 takes place on 2026-10-12 at 16:30 in B홀.",
            "행사 allows entry.",
            "In summary, its count is 8.",
        ] {
            assert!(english.markdown.contains(continuation), "{continuation}");
        }
    }

    #[test]
    fn subject_ellipsis_without_a_local_antecedent_fails_closed() {
        let response = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_LOCATION",
                "meeting",
                "회의",
                ApprovedRelationTypeIR::Location,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "room_b".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "회의실 B".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        assert!(interpret_document_semantics(
            "또한, 장소는 회의실 B입니다.",
            &response,
            LanguageCodeIR::Korean,
        )
        .is_err());
        assert!(interpret_document_semantics(
            "Also, its location is 회의실 B.",
            &response,
            LanguageCodeIR::English,
        )
        .is_err());
    }

    #[test]
    fn subject_ellipsis_stops_at_a_subject_change_and_resumes_locally() {
        let response = response(
            ApprovedDiscourseRelationIR::Explanation,
            ApprovedOperationIR::Explain,
            vec![
                claim(
                    "P_MEETING_TIME",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Time,
                    ApprovedOpenValueIR::Clock {
                        hour: 9,
                        minute: 30,
                    },
                ),
                claim(
                    "P_WORKSHOP_DATE",
                    "workshop",
                    "워크숍",
                    ApprovedRelationTypeIR::Date,
                    ApprovedOpenValueIR::Date {
                        year: 2026,
                        month: 10,
                        day: 12,
                    },
                ),
                claim(
                    "P_WORKSHOP_TIME",
                    "workshop",
                    "워크숍",
                    ApprovedRelationTypeIR::Time,
                    ApprovedOpenValueIR::Clock {
                        hour: 16,
                        minute: 30,
                    },
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );

        let korean = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(korean.validate(&response));
        assert!(korean
            .markdown
            .contains("1. 회의 시간은 오전 9시 30분입니다."));
        assert!(korean.markdown.contains(
            "2. 워크숍 날짜는 2026년 10월 12일입니다. 정리하면, 시간은 오후 4시 30분입니다."
        ));

        let english = realize_document_response(&response, LanguageCodeIR::English).unwrap();
        assert!(english.validate(&response));
        assert!(english.markdown.contains("1. The time of 회의 is 09:30."));
        assert!(english
            .markdown
            .contains("2. The date of 워크숍 is 2026-10-12. In summary, its time is 16:30."));
    }

    #[test]
    fn alternating_semantic_runs_are_merged_without_reordering_or_unbounded_blocks() {
        let claims = (0..16)
            .map(|index| {
                if index % 2 == 0 {
                    claim(
                        &format!("P_{index:02}"),
                        &format!("item_{index:02}"),
                        &format!("항목 {index}"),
                        ApprovedRelationTypeIR::Status,
                        ApprovedOpenValueIR::Text(format!("상태 {index}")),
                    )
                } else {
                    claim(
                        &format!("P_{index:02}"),
                        &format!("item_{index:02}"),
                        &format!("항목 {index}"),
                        ApprovedRelationTypeIR::Time,
                        ApprovedOpenValueIR::Clock {
                            hour: (index + 1) as u8,
                            minute: 0,
                        },
                    )
                }
            })
            .collect::<Vec<_>>();
        let response = response(
            ApprovedDiscourseRelationIR::Explanation,
            ApprovedOperationIR::Explain,
            claims,
            ApprovedVerbosityIR::Explanatory,
        );
        let output = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(output.validate(&response));
        assert!(output.plan.sections.len() <= MAX_SECTIONS);
        assert!(output.plan.blocks.len() <= MAX_BLOCKS);
        assert_eq!(
            output
                .plan
                .sections
                .iter()
                .flat_map(|section| section.claim_ids.iter().cloned())
                .collect::<Vec<_>>(),
            response
                .claims
                .iter()
                .map(|claim| claim.proposition_id.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn comparable_numbers_select_a_table_and_chart_from_typed_values() {
        let response = response(
            ApprovedDiscourseRelationIR::Comparison,
            ApprovedOperationIR::Compare,
            vec![
                claim(
                    "P_A",
                    "hall_a",
                    "A홀",
                    ApprovedRelationTypeIR::Capacity,
                    ApprovedOpenValueIR::Integer(120),
                ),
                claim(
                    "P_B",
                    "hall_b",
                    "B홀",
                    ApprovedRelationTypeIR::Capacity,
                    ApprovedOpenValueIR::Integer(80),
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );
        let output = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(output.validate(&response));
        assert!(output.markdown.contains("| 대상 | 항목 | 값 |"));
        assert!(output.markdown.contains("```mermaid"));
        assert!(output.markdown.contains("bar [120, 80]"));
        assert!(output
            .interpreted_structure
            .nodes
            .iter()
            .any(|node| node.kind == DocumentSurfaceNodeKindIR::Chart));
    }

    #[test]
    fn boolean_state_is_realized_as_a_korean_predicate() {
        let response = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_CANCELLED",
                "meeting",
                "회의",
                ApprovedRelationTypeIR::Cancelled,
                ApprovedOpenValueIR::Boolean(false),
            )],
            ApprovedVerbosityIR::Short,
        );
        let output = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert_eq!(output.markdown, "회의는 취소되지 않았습니다.");
        assert!(output.validate(&response));
        assert_eq!(
            output.semantic_interpretation.claims[0].value,
            ApprovedOpenValueIR::Boolean(false)
        );
    }

    #[test]
    fn semantic_inverse_rejects_a_changed_typed_value() {
        let response = response(
            ApprovedDiscourseRelationIR::Correction,
            ApprovedOperationIR::Revise,
            vec![claim(
                "P_TIME",
                "meeting",
                "회의",
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 0,
                },
            )],
            ApprovedVerbosityIR::Explanatory,
        );
        let output = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(output.markdown.contains("오후 4시로 변경됐습니다."));
        assert_eq!(output.semantic_interpretation.claims.len(), 1);
        let changed = output.markdown.replace("오후 4시", "오후 5시");
        assert!(interpret_document_semantics(&changed, &response, LanguageCodeIR::Korean).is_err());
    }

    #[test]
    fn english_inverse_recovers_date_and_quantity_types() {
        let unit = ApprovedLexicalNodeIR {
            node_id: "people".into(),
            semantic_type: ApprovedSemanticTypeIR::Unit,
            canonical_lexical_label: "people".into(),
        };
        let response = response(
            ApprovedDiscourseRelationIR::Explanation,
            ApprovedOperationIR::Explain,
            vec![
                claim(
                    "P_DATE",
                    "workshop",
                    "workshop",
                    ApprovedRelationTypeIR::Date,
                    ApprovedOpenValueIR::Date {
                        year: 2026,
                        month: 9,
                        day: 16,
                    },
                ),
                claim(
                    "P_CAPACITY",
                    "workshop",
                    "workshop",
                    ApprovedRelationTypeIR::Capacity,
                    ApprovedOpenValueIR::Quantity { amount: 30, unit },
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );
        let output = realize_document_response(&response, LanguageCodeIR::English).unwrap();
        assert!(output.validate(&response));
        assert!(output
            .semantic_interpretation
            .claims
            .iter()
            .any(|claim| matches!(claim.value, ApprovedOpenValueIR::Date { .. })));
        assert!(output
            .semantic_interpretation
            .claims
            .iter()
            .any(|claim| matches!(claim.value, ApprovedOpenValueIR::Quantity { .. })));
    }

    #[test]
    fn quoted_open_text_does_not_collapse_into_number_or_sentence_boundary() {
        let response = response(
            ApprovedDiscourseRelationIR::Correction,
            ApprovedOperationIR::Revise,
            vec![claim(
                "P_STATUS",
                "meeting",
                "회의",
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Text("123. 확인값".into()),
            )],
            ApprovedVerbosityIR::Explanatory,
        );
        let output = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        assert!(output.markdown.contains("“123. 확인값”으로 변경됐습니다."));
        assert_eq!(
            output.semantic_interpretation.claims[0].value,
            ApprovedOpenValueIR::Text("123. 확인값".into())
        );
        assert!(output.validate(&response));
    }

    #[test]
    fn bounded_long_document_roundtrips_configured_claim_limit() {
        let claims = (0..crate::approved_response::MAX_APPROVED_RESPONSE_CLAIMS)
            .map(|index| {
                claim(
                    &format!("P_{index:02}"),
                    &format!("item_{index:02}"),
                    &format!("항목 {index}"),
                    ApprovedRelationTypeIR::Count,
                    ApprovedOpenValueIR::Integer(index as i64),
                )
            })
            .collect::<Vec<_>>();
        let response = response(
            ApprovedDiscourseRelationIR::Explanation,
            ApprovedOperationIR::Explain,
            claims,
            ApprovedVerbosityIR::Explanatory,
        );
        let started = std::time::Instant::now();
        let output = realize_document_response(&response, LanguageCodeIR::Korean).unwrap();
        let elapsed = started.elapsed();
        assert!(output.validate(&response));
        assert_eq!(
            output.semantic_interpretation.claims.len(),
            crate::approved_response::MAX_APPROVED_RESPONSE_CLAIMS
        );
        assert!(!output.markdown.is_empty());
        assert!(output
            .plan
            .blocks
            .iter()
            .filter(|block| is_primary_content(block.kind))
            .all(|block| block.kind != DocumentResponseBlockKindIR::OrderedList));
        eprintln!(
            "B_CORE_DOCUMENT_BOUND_CLAIM_ROUNDTRIP={}",
            serde_json::to_string(&serde_json::json!({
                "status": "PASS",
                "claim_count": output.semantic_interpretation.claims.len(),
                "markdown_bytes": output.markdown.len(),
                "realize_and_semantic_inverse_millis": elapsed.as_secs_f64() * 1_000.0,
                "gpu_used": false,
            }))
            .unwrap()
        );

        let mut over_limit = response.clone();
        over_limit.claims.push(claim(
            &format!("P_{:02}", crate::approved_response::MAX_APPROVED_RESPONSE_CLAIMS),
            &format!("item_{:02}", crate::approved_response::MAX_APPROVED_RESPONSE_CLAIMS),
            &format!("항목 {}", crate::approved_response::MAX_APPROVED_RESPONSE_CLAIMS),
            ApprovedRelationTypeIR::Count,
            ApprovedOpenValueIR::Integer(
                crate::approved_response::MAX_APPROVED_RESPONSE_CLAIMS as i64,
            ),
        ));
        over_limit.semantic_sha256 = compositional_response_sha256(&over_limit);
        assert!(!over_limit.validate());
    }

    #[test]
    fn malformed_or_unclosed_structure_fails_closed() {
        assert!(interpret_document_surface("").is_err());
        assert!(interpret_document_surface("```mermaid\nxychart-beta").is_err());
    }

    #[test]
    fn cognitive_api_exposes_the_approved_document_product_path() {
        let approved = response(
            ApprovedDiscourseRelationIR::Temporal,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_TIME",
                "meeting",
                "회의",
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 0,
                },
            )],
            ApprovedVerbosityIR::Short,
        );
        let result = crate::cognitive::CognitiveApi::new_embedded()
            .unwrap()
            .execute_command(
                crate::cognitive::CognitiveApiCommandIR::RealizeApprovedDocumentResponse {
                    response: Box::new(approved.clone()),
                    output_language: LanguageCodeIR::Korean,
                },
            );
        let crate::cognitive::CognitiveApiPayloadIR::DocumentResponse(output) =
            result.payload.expect("document response payload")
        else {
            panic!("wrong cognitive API payload");
        };
        assert!(result.ok);
        assert!(output.validate(&approved));
        assert_eq!(output.markdown, "회의 시간은 오후 4시입니다.");
        let inverse = crate::cognitive::CognitiveApi::new_embedded()
            .unwrap()
            .execute_command(
                crate::cognitive::CognitiveApiCommandIR::InterpretApprovedDocumentResponse {
                    response: Box::new(approved.clone()),
                    output_language: LanguageCodeIR::Korean,
                    markdown: output.markdown.clone(),
                },
            );
        let crate::cognitive::CognitiveApiPayloadIR::DocumentSemanticInterpretation(interpretation) =
            inverse.payload.expect("semantic interpretation payload")
        else {
            panic!("wrong inverse cognitive API payload");
        };
        assert!(inverse.ok);
        assert!(interpretation.validate(&approved));
        assert_eq!(interpretation.recovered_claim_ids, vec!["P_TIME"]);
    }

    #[test]
    fn generic_speech_act_surfaces_roundtrip_claims() {
        let cases = [
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 0,
                },
                "회의 시간은 오후 4시인가요?",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                ApprovedRelationTypeIR::Registration,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "complete".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "완료".into(),
                }),
                "회의 등록 상태는 완료해 주세요.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "confirmed".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "확정".into(),
                }),
                "회의 상태는 확정하겠습니다.",
            ),
            (
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                ApprovedOperationIR::Reassure,
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "ready".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "준비".into(),
                }),
                "회의 상태는 준비예요.",
            ),
        ];
        for (speech_act, discourse_relation, operation, relation, value, surface) in cases {
            let mut response = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_SPEECH_ACT",
                    "MEETING",
                    "회의",
                    ApprovedRelationTypeIR::Status,
                    value,
                )],
                ApprovedVerbosityIR::Short,
            );
            response.claims[0].relation = relation;
            response.speech_act = speech_act;
            response.discourse_relation = discourse_relation;
            response.operation = operation;
            response.semantic_sha256 = compositional_response_sha256(&response);
            assert!(response.validate());
            let parsed = interpret_document_semantics(surface, &response, LanguageCodeIR::Korean)
                .expect("generic speech-act surface should parse");
            assert!(parsed.validate(&response));
            let rendered = realize_document_response(&response, LanguageCodeIR::Korean)
                .unwrap_or_else(|error| {
                    panic!("generic speech-act realization failed: {speech_act:?}: {error}")
                });
            assert!(rendered.validate(&response));
        }
    }

    #[test]
    fn casual_queries_select_the_copula_from_the_realized_value() {
        let cases = [
            (
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 30,
                },
                "회의 시간은 오후 4시 30분이야?",
            ),
            (
                ApprovedRelationTypeIR::Quantity,
                ApprovedOpenValueIR::Quantity {
                    amount: 3,
                    unit: ApprovedLexicalNodeIR {
                        node_id: "unit_item".into(),
                        semantic_type: ApprovedSemanticTypeIR::Unit,
                        canonical_lexical_label: "개".into(),
                    },
                },
                "회의 수량은 3개야?",
            ),
        ];
        for (relation, value, expected) in cases {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                vec![claim("P_CASUAL_QUERY", "meeting", "회의", relation, value)],
                ApprovedVerbosityIR::Short,
            );
            approved.speech_act = ApprovedSpeechActIR::Query;
            approved.style.register = LanguageRegisterIR::Informal;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("casual query should roundtrip");
            assert_eq!(rendered.markdown, expected);
        }
    }

    #[test]
    fn compact_korean_state_labels_realize_as_natural_state_nominals() {
        for (register, expected) in [
            (LanguageRegisterIR::Formal, "회의는 계획된 상태입니다."),
            (LanguageRegisterIR::Neutral, "회의는 계획된 상태예요."),
            (LanguageRegisterIR::Informal, "회의는 계획된 상태야."),
        ] {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_PLANNED_STATE",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Status,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "planned".into(),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: "계획됨".into(),
                    }),
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.style.register = register;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("natural state nominal should roundtrip");
            assert_eq!(rendered.markdown, expected, "register={register:?}");
            assert!(rendered.validate(&approved));
        }
    }

    #[test]
    fn eventive_arrival_state_labels_realize_as_attributive_states() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_PEAK_STATE",
                "starter_culture",
                "천연발효종",
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "peak_fermentation".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "최고 발효점 도달".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.style.register = LanguageRegisterIR::Neutral;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("eventive state label should roundtrip");
        assert_eq!(
            rendered.markdown,
            "천연발효종은 최고 발효점에 도달한 상태예요."
        );
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn compact_availability_confirmation_and_acquisition_states_are_inflected() {
        for (label, expected) in [
            ("예약 가능", "관람은 예약 가능한 상태예요."),
            ("화분괴 이동 확인", "관람은 화분괴 이동이 확인됐어요."),
            ("교체 자재 확보", "관람은 교체 자재가 확보됐어요."),
            ("기본 설계 검토 완료", "관람은 기본 설계 검토가 완료됐어요."),
            ("구조 보강 필요", "관람은 구조 보강이 필요해요."),
            ("온도 안정", "관람은 온도가 안정된 상태예요."),
            ("통과", "관람은 통과했어요."),
            ("필요", "관람이 필요해요."),
            ("미확인", "관람은 확인되지 않았어요."),
            ("조건부 완료", "관람은 조건부로 완료됐어요."),
            ("회원으로 제한", "관람은 회원으로 제한돼요."),
            ("성에 제거 완료", "관람은 성에 제거가 완료됐어요."),
            ("정격 용량 확보", "관람은 정격 용량이 확보됐어요."),
            ("통화 품질 확인", "관람은 통화 품질이 확인됐어요."),
            ("완료", "관람이 완료됐어요."),
            ("완료됨", "관람이 완료됐어요."),
            ("차단됨", "관람이 차단됐어요."),
            ("직접 침수 없음", "관람은 직접 침수가 없어요."),
            ("조건부 운영 가능", "관람은 조건부로 운영할 수 있어요."),
            ("조건부 정상 운영", "관람은 조건부로 정상 운영돼요."),
            ("현 위치 보존", "관람은 현 위치에 보존돼요."),
        ] {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_TYPED_STATUS",
                    "viewing",
                    "관람",
                    ApprovedRelationTypeIR::Status,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "typed_status".into(),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: label.into(),
                    }),
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.style.register = LanguageRegisterIR::Neutral;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("typed compact state should roundtrip");
            assert_eq!(rendered.markdown, expected, "label={label}");
            assert!(rendered.validate(&approved));
        }
    }

    #[test]
    fn lexical_restriction_status_omits_redundant_state_head() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_RESTRICTED_STATE",
                "gallery_access_state",
                "전시장 출입 상태",
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "restricted_to_deinstallation_staff".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "철수 인력으로 제한".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.style.register = LanguageRegisterIR::Formal;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("restriction status should roundtrip");
        assert_eq!(rendered.markdown, "전시장 출입은 철수 인력으로 제한됩니다.");
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn nominal_action_relation_uses_a_reversible_execution_frame() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_ACTION_FRAME",
                "signature_followup",
                "서명 누락 대응",
                ApprovedRelationTypeIR::Action,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "resend_signature_link".into(),
                    semantic_type: ApprovedSemanticTypeIR::Concept,
                    canonical_lexical_label: "전자서명 링크 재발송".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.style.register = LanguageRegisterIR::Formal;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("action relation should roundtrip");
        assert_eq!(
            rendered.markdown,
            "서명 누락 대응 조치는 전자서명 링크 재발송으로 진행됩니다."
        );
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn directional_nominal_actions_avoid_a_doubled_execution_particle() {
        for (subject, label, expected) in [
            (
                "옛 매표소",
                "지역 기록 열람실로 전환",
                "옛 매표소 조치는 지역 기록 열람실로 전환하는 방식으로 진행됩니다.",
            ),
            (
                "주민 전시 공간",
                "분기별 공개 모집으로 운영",
                "주민 전시 공간 조치는 분기별 공개 모집으로 운영하는 방식으로 진행됩니다.",
            ),
            (
                "승강기 설치",
                "기존 계단실 후면에 독립 구조로 시공",
                "승강기 설치 조치는 기존 계단실 후면에 독립 구조로 시공하는 방식으로 진행됩니다.",
            ),
        ] {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_DIRECTIONAL_ACTION",
                    "directional_action",
                    subject,
                    ApprovedRelationTypeIR::Action,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "directional_action_value".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: label.into(),
                    }),
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.style.register = LanguageRegisterIR::Formal;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("directional nominal action should roundtrip");
            assert_eq!(rendered.markdown, expected, "label={label}");
            assert!(rendered.validate(&approved));
        }
    }

    #[test]
    fn absence_status_uses_a_result_context_when_the_subject_supplies_one() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_ABSENCE_STATUS",
                "inspection_result_state",
                "1차 점검 결과 상태",
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "no_direct_flooding".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "직접 침수 없음".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.style.register = LanguageRegisterIR::Formal;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("absence state should roundtrip");
        assert_eq!(rendered.markdown, "1차 점검에서는 직접 침수가 없습니다.");
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn embedded_location_subjects_use_a_locative_clause_without_an_inferred_event() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_EMBEDDED_LOCATION",
                "accessible_route_location",
                "무장애 접근로 장소",
                ApprovedRelationTypeIR::Location,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "south_gentle_slope".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "남측 완만한 경사 구간".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.style.register = LanguageRegisterIR::Formal;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("embedded location should roundtrip");
        assert_eq!(
            rendered.markdown,
            "무장애 접근로는 남측 완만한 경사 구간에 있습니다."
        );
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn compound_location_heads_keep_their_nominal_head() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_COMPOUND_LOCATION",
                "specimen_storage_location",
                "검체 이송 상자 보관 위치",
                ApprovedRelationTypeIR::Location,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "east_elevator_waiting_area".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "동측 전용 승강기 앞 대기 구역".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.style.register = LanguageRegisterIR::Formal;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("compound locative head should roundtrip");
        assert_eq!(
            rendered.markdown,
            "검체 이송 상자 보관 위치는 동측 전용 승강기 앞 대기 구역에 있습니다."
        );
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn typed_concrete_location_subjects_use_the_locative_clause() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_CONCRETE_LOCATION",
                "shade_screen",
                "차광막",
                ApprovedRelationTypeIR::Location,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "hive_top".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "벌통 위 30센티미터 높이".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.claims[0].subject.semantic_type = ApprovedSemanticTypeIR::Concept;
        approved.style.register = LanguageRegisterIR::Formal;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("typed concrete location should roundtrip");
        assert_eq!(
            rendered.markdown,
            "차광막은 벌통 위 30센티미터 높이에 있습니다."
        );
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn status_dimension_subjects_do_not_gain_a_redundant_state_head() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_INTERVAL",
                "water_replacement_interval",
                "물 교체 주기",
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "daily_once".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "매일 1회".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.style.register = LanguageRegisterIR::Neutral;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("status dimension should roundtrip");
        assert_eq!(rendered.markdown, "물 교체 주기는 매일 1회예요.");
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn impact_need_labels_use_the_approved_causal_relation_without_an_action_guess() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_IMPACT_NEED",
                "burial_depth_impact",
                "매설 심도 영향",
                ApprovedRelationTypeIR::Impact,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "contact_risk_reduction_need".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "향후 어구 접촉 위험 감소 필요".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.style.register = LanguageRegisterIR::Formal;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("impact need should roundtrip");
        assert_eq!(
            rendered.markdown,
            "매설 심도의 영향으로 향후 어구 접촉 위험 감소가 필요합니다."
        );
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn contextual_status_predicates_preserve_their_roles_when_reordered() {
        for (subject, label, expected) in [
            (
                "종자 출고 제한",
                "복구 검증 전 일시 중지",
                "종자 출고 제한은 복구 검증 전까지 일시 중지됩니다.",
            ),
            (
                "야간 소음 기준",
                "주거지 경계 측정값 준수",
                "주거지 경계 측정값은 야간 소음 기준을 준수합니다.",
            ),
            (
                "역사 외벽",
                "원형 벽돌 보존",
                "역사 외벽의 원형 벽돌은 보존됩니다.",
            ),
            (
                "다음 회의 안건",
                "운영 주체와 대관 기준 확정",
                "다음 회의 안건은 운영 주체와 대관 기준을 확정하는 것입니다.",
            ),
            (
                "문화재 자문",
                "다음 설계안 제출 전 필요",
                "문화재 자문은 다음 설계안 제출 전에 필요합니다.",
            ),
        ] {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_CONTEXTUAL_STATUS",
                    "contextual_status",
                    subject,
                    ApprovedRelationTypeIR::Status,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "contextual_state".into(),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: label.into(),
                    }),
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.style.register = LanguageRegisterIR::Formal;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("contextual state should roundtrip");
            assert_eq!(rendered.markdown, expected, "label={label}");
            assert!(rendered.validate(&approved));
        }
    }

    #[test]
    fn relational_subject_nominals_do_not_repeat_the_same_korean_relation() {
        let cases = [
            (
                "초기 차수 조치",
                ApprovedRelationTypeIR::Action,
                "방수포 설치",
                "초기 차수 조치는 방수포 설치로 진행됩니다.",
            ),
            (
                "격리 작업 담당",
                ApprovedRelationTypeIR::Owner,
                "종자 보존팀",
                "격리 작업 담당자는 종자 보존팀입니다.",
            ),
            (
                "차광막 위치",
                ApprovedRelationTypeIR::Location,
                "벌통 위 30센티미터",
                "차광막은 벌통 위 30센티미터에 있습니다.",
            ),
            (
                "작품 임시 집결지",
                ApprovedRelationTypeIR::Location,
                "지하 1층 포장실",
                "작품 임시 집결지는 지하 1층 포장실입니다.",
            ),
            (
                "주차 수요 영향",
                ApprovedRelationTypeIR::Impact,
                "주말 골목 혼잡",
                "주차 수요 영향은 주말 골목 혼잡입니다.",
            ),
            (
                "국제 판화 특별전 철수",
                ApprovedRelationTypeIR::Cause,
                "전시 운영 기간 종료",
                "국제 판화 특별전 철수의 원인은 전시 운영 기간 종료입니다.",
            ),
            (
                "상설 전시 자료 출처",
                ApprovedRelationTypeIR::Source,
                "지역 철도 노동자 구술 기록",
                "상설 전시 자료 출처는 지역 철도 노동자 구술 기록입니다.",
            ),
        ];
        for (subject, relation, value, expected) in cases {
            let approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_RELATIONAL_SUBJECT",
                    "relational_subject",
                    subject,
                    relation,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "relational_value".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: value.into(),
                    }),
                )],
                ApprovedVerbosityIR::Short,
            );
            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("embedded relation should roundtrip");
            assert_eq!(rendered.markdown, expected, "subject={subject}");
            assert!(rendered.validate(&approved));
        }
    }

    #[test]
    fn multi_field_query_roundtrips_as_one_approved_question_set() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Inquiry,
            ApprovedOperationIR::Query,
            vec![
                claim(
                    "P_QUERY_STATUS",
                    "lunar_viewing",
                    "월면 시료 공개 관람",
                    ApprovedRelationTypeIR::Status,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "reservation_open".into(),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: "예약 가능".into(),
                    }),
                ),
                claim(
                    "P_QUERY_DATE",
                    "lunar_viewing",
                    "월면 시료 공개 관람",
                    ApprovedRelationTypeIR::Date,
                    ApprovedOpenValueIR::Date {
                        year: 2026,
                        month: 10,
                        day: 11,
                    },
                ),
                claim(
                    "P_QUERY_TIME",
                    "lunar_viewing",
                    "월면 시료 공개 관람",
                    ApprovedRelationTypeIR::Time,
                    ApprovedOpenValueIR::Clock {
                        hour: 13,
                        minute: 20,
                    },
                ),
                claim(
                    "P_QUERY_LOCATION",
                    "lunar_viewing",
                    "월면 시료 공개 관람",
                    ApprovedRelationTypeIR::Location,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "clean_gallery".into(),
                        semantic_type: ApprovedSemanticTypeIR::Location,
                        canonical_lexical_label: "청정 전시실".into(),
                    }),
                ),
                claim(
                    "P_QUERY_CAPACITY",
                    "lunar_viewing",
                    "월면 시료 공개 관람",
                    ApprovedRelationTypeIR::Capacity,
                    ApprovedOpenValueIR::Quantity {
                        amount: 12,
                        unit: ApprovedLexicalNodeIR {
                            node_id: "person_unit".into(),
                            semantic_type: ApprovedSemanticTypeIR::Unit,
                            canonical_lexical_label: "명".into(),
                        },
                    },
                ),
            ],
            ApprovedVerbosityIR::Explanatory,
        );
        approved.speech_act = ApprovedSpeechActIR::Query;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let plan = build_document_response_plan(&approved, LanguageCodeIR::Korean).unwrap();
        let markdown = render_plan(&plan, &approved);
        let interpreted =
            interpret_document_semantics(&markdown, &approved, LanguageCodeIR::Korean);
        assert!(interpreted.is_ok(), "{markdown}\n{interpreted:?}");
        assert!(interpreted.unwrap().validate(&approved));
    }

    #[test]
    fn count_units_select_copulas_and_directional_particles_from_the_unit_coda() {
        for (surface, expected) in [
            ("0", "0으로"),
            ("1", "1로"),
            ("6", "6으로"),
            ("7", "7로"),
            ("8", "8로"),
            ("9", "9로"),
            ("파일", "파일로"),
            ("점검", "점검으로"),
        ] {
            assert_eq!(with_directional_particle(surface), expected, "{surface}");
        }
        assert_eq!(
            korean_count_speech_act_clause(
                "미배정 문의",
                23,
                "건",
                ApprovedSpeechActIR::Inform,
                ApprovedOperationIR::Assert,
                LanguageRegisterIR::Neutral,
            ),
            "미배정 문의는 23건이에요"
        );
        assert_eq!(
            korean_count_speech_act_clause(
                "미배정 문의",
                23,
                "건",
                ApprovedSpeechActIR::Inform,
                ApprovedOperationIR::Assert,
                LanguageRegisterIR::Informal,
            ),
            "미배정 문의는 23건이야"
        );
        assert_eq!(
            korean_count_speech_act_clause(
                "항목",
                3,
                "개",
                ApprovedSpeechActIR::Inform,
                ApprovedOperationIR::Assert,
                LanguageRegisterIR::Neutral,
            ),
            "항목은 3개예요"
        );
        assert_eq!(
            korean_count_speech_act_clause(
                "미배정 문의",
                23,
                "건",
                ApprovedSpeechActIR::Request,
                ApprovedOperationIR::Request,
                LanguageRegisterIR::Neutral,
            ),
            "미배정 문의 수량을 23건으로 맞춰 주세요"
        );
        assert_eq!(
            korean_count_speech_act_clause(
                "미배정 문의",
                23,
                "건",
                ApprovedSpeechActIR::Inform,
                ApprovedOperationIR::Revise,
                LanguageRegisterIR::Neutral,
            ),
            "미배정 문의는 23건으로 바뀌었어요"
        );
    }

    #[test]
    fn korean_document_grammar_matrix_roundtrips_all_relation_act_register_and_polarity_axes() {
        fn lexical(
            id: &str,
            label: &str,
            semantic_type: ApprovedSemanticTypeIR,
        ) -> ApprovedOpenValueIR {
            ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                node_id: id.into(),
                semantic_type,
                canonical_lexical_label: label.into(),
            })
        }
        let cases = vec![
            (
                ApprovedRelationTypeIR::Status,
                lexical("ready", "준비", ApprovedSemanticTypeIR::State),
            ),
            (
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 30,
                },
            ),
            (
                ApprovedRelationTypeIR::Cancelled,
                ApprovedOpenValueIR::Boolean(true),
            ),
            (
                ApprovedRelationTypeIR::EarlierThan,
                lexical("review", "검토", ApprovedSemanticTypeIR::Event),
            ),
            (
                ApprovedRelationTypeIR::RoomAvailable,
                ApprovedOpenValueIR::Boolean(true),
            ),
            (
                ApprovedRelationTypeIR::Location,
                lexical("room", "회의실", ApprovedSemanticTypeIR::Location),
            ),
            (
                ApprovedRelationTypeIR::Registration,
                ApprovedOpenValueIR::Boolean(true),
            ),
            (
                ApprovedRelationTypeIR::Entry,
                ApprovedOpenValueIR::Boolean(true),
            ),
            (
                ApprovedRelationTypeIR::Confirmed,
                ApprovedOpenValueIR::Boolean(true),
            ),
            (
                ApprovedRelationTypeIR::Approved,
                ApprovedOpenValueIR::Boolean(true),
            ),
            (
                ApprovedRelationTypeIR::Capacity,
                ApprovedOpenValueIR::Integer(6),
            ),
            (
                ApprovedRelationTypeIR::Duration,
                ApprovedOpenValueIR::Quantity {
                    amount: 15,
                    unit: ApprovedLexicalNodeIR {
                        node_id: "minute".into(),
                        semantic_type: ApprovedSemanticTypeIR::Unit,
                        canonical_lexical_label: "분".into(),
                    },
                },
            ),
            (
                ApprovedRelationTypeIR::Name,
                ApprovedOpenValueIR::Text("정기 점검".into()),
            ),
            (
                ApprovedRelationTypeIR::Count,
                ApprovedOpenValueIR::Integer(8),
            ),
            (
                ApprovedRelationTypeIR::Date,
                ApprovedOpenValueIR::Date {
                    year: 2026,
                    month: 9,
                    day: 23,
                },
            ),
            (
                ApprovedRelationTypeIR::Quantity,
                ApprovedOpenValueIR::Quantity {
                    amount: 23,
                    unit: ApprovedLexicalNodeIR {
                        node_id: "case".into(),
                        semantic_type: ApprovedSemanticTypeIR::Unit,
                        canonical_lexical_label: "건".into(),
                    },
                },
            ),
            (
                ApprovedRelationTypeIR::Cause,
                lexical("failure", "장애", ApprovedSemanticTypeIR::Event),
            ),
            (
                ApprovedRelationTypeIR::Impact,
                lexical("delay", "지연", ApprovedSemanticTypeIR::State),
            ),
            (
                ApprovedRelationTypeIR::Action,
                lexical("rollback", "롤백", ApprovedSemanticTypeIR::Event),
            ),
            (
                ApprovedRelationTypeIR::Owner,
                lexical("team", "운영팀", ApprovedSemanticTypeIR::Organization),
            ),
            (
                ApprovedRelationTypeIR::Deadline,
                ApprovedOpenValueIR::Clock {
                    hour: 18,
                    minute: 0,
                },
            ),
            (
                ApprovedRelationTypeIR::Agent,
                lexical("worker", "담당자", ApprovedSemanticTypeIR::Person),
            ),
            (
                ApprovedRelationTypeIR::Patient,
                lexical("sample", "시료", ApprovedSemanticTypeIR::Concept),
            ),
            (
                ApprovedRelationTypeIR::Theme,
                lexical("file", "파일", ApprovedSemanticTypeIR::Concept),
            ),
            (
                ApprovedRelationTypeIR::Source,
                lexical("lab", "준비실", ApprovedSemanticTypeIR::Location),
            ),
            (
                ApprovedRelationTypeIR::Destination,
                lexical("zone", "격리 구역", ApprovedSemanticTypeIR::Location),
            ),
            (
                ApprovedRelationTypeIR::Instrument,
                lexical("cart", "운반 카트", ApprovedSemanticTypeIR::Concept),
            ),
            (
                ApprovedRelationTypeIR::Manner,
                lexical("careful", "신중", ApprovedSemanticTypeIR::Concept),
            ),
            (
                ApprovedRelationTypeIR::Target,
                lexical("server", "서버", ApprovedSemanticTypeIR::Concept),
            ),
            (
                ApprovedRelationTypeIR::InitialState,
                lexical("closed", "폐쇄", ApprovedSemanticTypeIR::State),
            ),
            (
                ApprovedRelationTypeIR::ResultState,
                lexical("open", "개방", ApprovedSemanticTypeIR::State),
            ),
        ];
        let acts = [
            (
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                ApprovedModalityIR::Asserted,
            ),
            (
                ApprovedSpeechActIR::Acknowledge,
                ApprovedDiscourseRelationIR::Confirmation,
                ApprovedOperationIR::Confirm,
                ApprovedModalityIR::Asserted,
            ),
            (
                ApprovedSpeechActIR::Explain,
                ApprovedDiscourseRelationIR::Explanation,
                ApprovedOperationIR::Explain,
                ApprovedModalityIR::Asserted,
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                ApprovedModalityIR::Interrogative,
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                ApprovedModalityIR::Directive,
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
                ApprovedModalityIR::Commissive,
            ),
            (
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                ApprovedOperationIR::Reassure,
                ApprovedModalityIR::Reassuring,
            ),
        ];
        let registers = [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
            LanguageRegisterIR::Internet,
        ];
        let mut audited = 0usize;
        for (case_index, (relation, value)) in cases.into_iter().enumerate() {
            let (subject_id, subject_label) = if case_index % 2 == 0 {
                ("inspection", "점검")
            } else {
                ("deployment", "배포")
            };
            for (speech_act, discourse, operation, modality) in acts {
                for register in registers {
                    for polarity in [true, false] {
                        let mut approved = response(
                            discourse,
                            operation,
                            vec![claim(
                                "P_GRAMMAR_MATRIX",
                                subject_id,
                                subject_label,
                                relation,
                                value.clone(),
                            )],
                            ApprovedVerbosityIR::Short,
                        );
                        approved.speech_act = speech_act;
                        approved.claims[0].polarity = polarity;
                        approved.claims[0].modality = modality;
                        approved.style.register = register;
                        approved.semantic_sha256 = compositional_response_sha256(&approved);
                        assert!(
                            approved.validate(),
                            "{relation:?}/{speech_act:?}/{register:?}/{polarity}"
                        );
                        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                            .unwrap_or_else(|error| {
                                panic!(
                                    "{relation:?}/{speech_act:?}/{register:?}/{polarity}: {error}"
                                )
                            });
                        assert!(rendered.validate(&approved));
                        for token in rendered.markdown.split_whitespace() {
                            let token = token.trim_matches(|character: char| {
                                matches!(
                                    character,
                                    '.' | ',' | '?' | '!' | ':' | ';' | '(' | ')' | '[' | ']'
                                )
                            });
                            if let Some(analysis) = crate::korean_copula::analyze(token) {
                                assert!(
                                    !analysis.particle_repair,
                                    "surface={} token={token}",
                                    rendered.markdown
                                );
                            }
                        }
                        for malformed in [
                            "건예요",
                            "건야",
                            "건로 ",
                            "분예요",
                            "분야",
                            "분로 ",
                            "명예요",
                            "명야",
                            "명로 ",
                        ] {
                            assert!(
                                !rendered.markdown.contains(malformed),
                                "surface={} malformed={malformed}",
                                rendered.markdown
                            );
                        }
                        audited += 1;
                    }
                }
            }
        }
        assert_eq!(audited, 31 * 7 * 4 * 2);
    }

    #[test]
    fn typed_korean_state_constructions_realize_and_roundtrip_naturally() {
        let cases = [
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                ApprovedRelationTypeIR::Status,
                "ready",
                "준비",
                "회의는 준비됐습니까?",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                ApprovedRelationTypeIR::Status,
                "ready",
                "준비",
                "회의를 준비해 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
                ApprovedRelationTypeIR::Status,
                "ready",
                "준비",
                "회의를 준비하겠습니다.",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                ApprovedRelationTypeIR::Registration,
                "complete",
                "완료",
                "회의 등록이 완료됐습니까?",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                ApprovedRelationTypeIR::Registration,
                "complete",
                "완료",
                "회의 등록을 완료해 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
                ApprovedRelationTypeIR::Registration,
                "complete",
                "완료",
                "회의 등록을 완료하겠습니다.",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                ApprovedRelationTypeIR::Status,
                "in_progress",
                "진행 중",
                "회의 상태를 진행 중으로 설정해 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
                ApprovedRelationTypeIR::Status,
                "in_progress",
                "진행 중",
                "회의 상태를 진행 중으로 설정하겠습니다.",
            ),
        ];

        for (speech_act, discourse, operation, relation, node_id, label, expected) in cases {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_TYPED_STATE",
                    "meeting",
                    "회의",
                    relation,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: node_id.into(),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: label.into(),
                    }),
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.speech_act = speech_act;
            approved.discourse_relation = discourse;
            approved.operation = operation;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            assert!(approved.validate());

            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("typed state realization should roundtrip");
            assert_eq!(rendered.markdown, expected);
            assert!(rendered.validate(&approved));
        }
    }

    #[test]
    fn typed_state_codec_transfers_to_unseen_subject_families() {
        let cases = [
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                ApprovedRelationTypeIR::Status,
                "ready",
                "준비",
                "현장 교육",
                "현장 교육은 준비됐습니까?",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                ApprovedRelationTypeIR::Status,
                "ready",
                "준비",
                "제품 시연",
                "제품 시연을 준비해 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
                ApprovedRelationTypeIR::Status,
                "ready",
                "준비",
                "안전 점검",
                "안전 점검을 준비하겠습니다.",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                ApprovedRelationTypeIR::Registration,
                "complete",
                "완료",
                "세미나",
                "세미나 등록이 완료됐습니까?",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                ApprovedRelationTypeIR::Registration,
                "complete",
                "완료",
                "전시",
                "전시 등록을 완료해 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
                ApprovedRelationTypeIR::Registration,
                "complete",
                "완료",
                "견학",
                "견학 등록을 완료하겠습니다.",
            ),
        ];

        for (speech_act, discourse, operation, relation, node_id, label, subject, expected) in cases {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_TYPED_STATE_TRANSFER",
                    "UNSEEN_SUBJECT",
                    subject,
                    relation,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: node_id.into(),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: label.into(),
                    }),
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.speech_act = speech_act;
            approved.discourse_relation = discourse;
            approved.operation = operation;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            assert!(approved.validate(), "{subject}/{speech_act:?}");

            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("typed state transfer should roundtrip");
            assert_eq!(rendered.markdown, expected, "{subject}/{speech_act:?}");
            assert!(rendered.validate(&approved), "{subject}/{speech_act:?}");
        }
    }
    #[test]
    fn korean_directional_complement_handles_consonants_and_rieul() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Directive,
            ApprovedOperationIR::Request,
            vec![claim(
                "P_TIME_REQUEST",
                "class",
                "수업",
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 10,
                    minute: 15,
                },
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.speech_act = ApprovedSpeechActIR::Request;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("request complement should roundtrip");
        assert_eq!(
            rendered.markdown,
            "수업 시간을 오전 10시 15분으로 해 주십시오."
        );
        assert!(rendered.validate(&approved));

        let mut rieul = response(
            ApprovedDiscourseRelationIR::Directive,
            ApprovedOperationIR::Request,
            vec![claim(
                "P_LOCATION_REQUEST",
                "meeting",
                "회의",
                ApprovedRelationTypeIR::Location,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "room_beta".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "베타실".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        rieul.speech_act = ApprovedSpeechActIR::Request;
        rieul.semantic_sha256 = compositional_response_sha256(&rieul);
        let rendered = realize_document_response(&rieul, LanguageCodeIR::Korean)
            .expect("rieul-final request complement should roundtrip");
        assert_eq!(rendered.markdown, "회의 장소를 베타실로 해 주십시오.");
        assert!(rendered.validate(&rieul));
    }

    #[test]
    fn reassurance_marker_is_preserved_across_korean_registers() {
        for register in [
            LanguageRegisterIR::Formal,
            LanguageRegisterIR::Neutral,
            LanguageRegisterIR::Informal,
            LanguageRegisterIR::Internet,
        ] {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Reassurance,
                ApprovedOperationIR::Reassure,
                vec![claim(
                    "P_REASSURE_TIME",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::Time,
                    ApprovedOpenValueIR::Clock {
                        hour: 16,
                        minute: 30,
                    },
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.speech_act = ApprovedSpeechActIR::Reassure;
            approved.style.register = register;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("reassurance marker should roundtrip");
            let expected_prefix = format!("{} ", korean_reassurance_marker(register));
            assert!(
                rendered.markdown.starts_with(&expected_prefix),
                "{}",
                rendered.markdown
            );
            assert!(rendered.validate(&approved));
        }
    }

    #[test]
    fn integer_measurements_use_relation_bound_korean_classifiers() {
        let cases = [
            (ApprovedRelationTypeIR::Capacity, "event", "행사", 6, "6명"),
            (ApprovedRelationTypeIR::Count, "item", "항목", 8, "8개"),
        ];
        let acts = [
            (
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
            ),
        ];

        for (relation, subject_id, subject_label, amount, classified) in cases {
            for (speech_act, discourse, operation) in acts {
                for register in [
                    LanguageRegisterIR::Formal,
                    LanguageRegisterIR::Neutral,
                    LanguageRegisterIR::Informal,
                ] {
                    let mut approved = response(
                        ApprovedDiscourseRelationIR::Statement,
                        ApprovedOperationIR::Assert,
                        vec![claim(
                            "P_CLASSIFIED_INTEGER",
                            subject_id,
                            subject_label,
                            relation,
                            ApprovedOpenValueIR::Integer(amount),
                        )],
                        ApprovedVerbosityIR::Short,
                    );
                    approved.speech_act = speech_act;
                    approved.discourse_relation = discourse;
                    approved.operation = operation;
                    approved.style.register = register;
                    approved.semantic_sha256 = compositional_response_sha256(&approved);

                    let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                        .unwrap_or_else(|error| {
                            panic!("{speech_act:?}/{relation:?}/{register:?}: {error}")
                        });
                    assert!(
                        rendered.markdown.contains(classified),
                        "{}",
                        rendered.markdown
                    );
                    assert!(rendered.validate(&approved));
                }
            }
        }
    }

    #[test]
    fn temporal_ordering_uses_relational_predicates_across_speech_acts() {
        let acts = [
            (
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
            ),
            (
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                ApprovedOperationIR::Reassure,
            ),
        ];

        for (speech_act, discourse, operation) in acts {
            for holds in [true, false] {
                for register in [
                    LanguageRegisterIR::Formal,
                    LanguageRegisterIR::Neutral,
                    LanguageRegisterIR::Informal,
                    LanguageRegisterIR::Internet,
                ] {
                    let mut approved = response(
                        ApprovedDiscourseRelationIR::Statement,
                        ApprovedOperationIR::Assert,
                        vec![claim(
                            "P_EARLIER_THAN",
                            "meeting",
                            "회의",
                            ApprovedRelationTypeIR::EarlierThan,
                            ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                                node_id: "class".into(),
                                semantic_type: ApprovedSemanticTypeIR::Event,
                                canonical_lexical_label: "수업".into(),
                            }),
                        )],
                        ApprovedVerbosityIR::Short,
                    );
                    approved.claims[0].polarity = holds;
                    approved.speech_act = speech_act;
                    approved.discourse_relation = discourse;
                    approved.operation = operation;
                    approved.style.register = register;
                    approved.semantic_sha256 = compositional_response_sha256(&approved);
                    assert!(approved.validate());

                    let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                        .unwrap_or_else(|error| {
                            panic!("{speech_act:?}/{holds}/{register:?}: {error}")
                        });
                    assert!(rendered.validate(&approved));
                    assert!(!rendered.markdown.contains("선행 관계"));
                }
            }
        }
    }

    #[test]
    fn temporal_ordering_registry_has_natural_surfaces() {
        let cases = [
            (
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                LanguageRegisterIR::Formal,
                true,
                "회의는 수업보다 앞섭니다.",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                LanguageRegisterIR::Neutral,
                true,
                "회의는 수업보다 앞서나요?",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                LanguageRegisterIR::Neutral,
                true,
                "회의를 수업보다 앞서 배치해 주세요.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
                LanguageRegisterIR::Formal,
                false,
                "회의를 수업보다 앞서 배치하지 않겠습니다.",
            ),
        ];

        for (speech_act, discourse, operation, register, holds, expected) in cases {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_EARLIER_SURFACE",
                    "meeting",
                    "회의",
                    ApprovedRelationTypeIR::EarlierThan,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "class".into(),
                        semantic_type: ApprovedSemanticTypeIR::Event,
                        canonical_lexical_label: "수업".into(),
                    }),
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.claims[0].polarity = holds;
            approved.speech_act = speech_act;
            approved.discourse_relation = discourse;
            approved.operation = operation;
            approved.style.register = register;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("temporal ordering surface should roundtrip");
            assert_eq!(rendered.markdown, expected);
        }
    }

    #[test]
    fn negative_open_values_preserve_the_requested_speech_act() {
        let cases = [
            (
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "in_progress".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "진행 중".into(),
                }),
            ),
            (
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 30,
                },
            ),
            (
                ApprovedRelationTypeIR::Location,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "room_beta".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "베타실".into(),
                }),
            ),
            (
                ApprovedRelationTypeIR::Capacity,
                ApprovedOpenValueIR::Integer(12),
            ),
            (
                ApprovedRelationTypeIR::Name,
                ApprovedOpenValueIR::Text("가을 워크숍".into()),
            ),
        ];
        let acts = [
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
            ),
            (
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                ApprovedOperationIR::Reassure,
            ),
        ];

        for (relation, value) in cases {
            for (speech_act, discourse, operation) in acts {
                for register in [
                    LanguageRegisterIR::Formal,
                    LanguageRegisterIR::Neutral,
                    LanguageRegisterIR::Informal,
                    LanguageRegisterIR::Internet,
                ] {
                    let mut approved = response(
                        ApprovedDiscourseRelationIR::Statement,
                        ApprovedOperationIR::Assert,
                        vec![claim(
                            "P_NEGATIVE_OPEN_VALUE",
                            "meeting",
                            "회의",
                            relation,
                            value.clone(),
                        )],
                        ApprovedVerbosityIR::Short,
                    );
                    approved.claims[0].polarity = false;
                    approved.speech_act = speech_act;
                    approved.discourse_relation = discourse;
                    approved.operation = operation;
                    approved.style.register = register;
                    approved.semantic_sha256 = compositional_response_sha256(&approved);
                    let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                        .unwrap_or_else(|error| {
                            panic!("{speech_act:?}/{relation:?}/{register:?}: {error}")
                        });
                    assert!(rendered.validate(&approved));
                    match speech_act {
                        ApprovedSpeechActIR::Query => assert!(rendered.markdown.ends_with('?')),
                        ApprovedSpeechActIR::Request => {
                            assert!(
                                rendered.markdown.contains("말아")
                                    || rendered.markdown.contains("마.")
                            )
                        }
                        ApprovedSpeechActIR::Promise => assert!(rendered.markdown.contains("않")),
                        ApprovedSpeechActIR::Reassure => {
                            let expected_prefix =
                                format!("{} ", korean_reassurance_marker(register));
                            assert!(rendered.markdown.starts_with(&expected_prefix))
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    #[test]
    fn negative_open_value_registry_has_natural_surfaces() {
        let cases = [
            (
                ApprovedSpeechActIR::Inform,
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "ready".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "준비".into(),
                }),
                "회의는 준비되지 않았습니다.",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "ready".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "준비".into(),
                }),
                "회의는 준비되지 않았습니까?",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "ready".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "준비".into(),
                }),
                "회의를 준비하지 말아 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "ready".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "준비".into(),
                }),
                "회의를 준비하지 않겠습니다.",
            ),
            (
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                ApprovedOperationIR::Reassure,
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "ready".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "준비".into(),
                }),
                "안심하셔도 됩니다. 회의는 준비되지 않았습니다.",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                ApprovedRelationTypeIR::Registration,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "complete".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "완료".into(),
                }),
                "회의 등록이 완료되지 않았습니까?",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                ApprovedRelationTypeIR::Registration,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "complete".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "완료".into(),
                }),
                "회의 등록을 완료하지 말아 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
                ApprovedRelationTypeIR::Time,
                ApprovedOpenValueIR::Clock {
                    hour: 16,
                    minute: 30,
                },
                "회의 시간은 오후 4시 30분이 아닙니까?",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
                ApprovedRelationTypeIR::Location,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "room_beta".into(),
                    semantic_type: ApprovedSemanticTypeIR::Location,
                    canonical_lexical_label: "베타실".into(),
                }),
                "회의 장소를 베타실로 하지 말아 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "in_progress".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "진행 중".into(),
                }),
                "회의 상태를 진행 중으로 설정하지 않겠습니다.",
            ),
        ];

        for (speech_act, discourse, operation, relation, value, expected) in cases {
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_NEGATIVE_SURFACE",
                    "meeting",
                    "회의",
                    relation,
                    value,
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.claims[0].polarity = false;
            approved.speech_act = speech_act;
            approved.discourse_relation = discourse;
            approved.operation = operation;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("negative open-value surface should roundtrip");
            assert_eq!(rendered.markdown, expected);
        }
    }

    #[test]
    fn boolean_relations_use_speech_act_predicates_instead_of_truth_nouns() {
        let relations = [
            (ApprovedRelationTypeIR::Cancelled, "meeting", "회의"),
            (
                ApprovedRelationTypeIR::RoomAvailable,
                "meeting_room",
                "회의실",
            ),
            (ApprovedRelationTypeIR::Registration, "workshop", "워크숍"),
            (ApprovedRelationTypeIR::Entry, "workshop", "워크숍"),
            (ApprovedRelationTypeIR::Confirmed, "meeting", "회의"),
            (ApprovedRelationTypeIR::Approved, "budget", "교육 예산"),
        ];
        let acts = [
            (
                ApprovedSpeechActIR::Query,
                ApprovedDiscourseRelationIR::Inquiry,
                ApprovedOperationIR::Query,
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedDiscourseRelationIR::Directive,
                ApprovedOperationIR::Request,
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedDiscourseRelationIR::Commitment,
                ApprovedOperationIR::Promise,
            ),
            (
                ApprovedSpeechActIR::Reassure,
                ApprovedDiscourseRelationIR::Reassurance,
                ApprovedOperationIR::Reassure,
            ),
        ];

        for (relation, subject_id, subject_label) in relations {
            for asserted in [true, false] {
                for (speech_act, discourse, operation) in acts {
                    for register in [
                        LanguageRegisterIR::Formal,
                        LanguageRegisterIR::Neutral,
                        LanguageRegisterIR::Informal,
                        LanguageRegisterIR::Internet,
                    ] {
                        let mut approved = response(
                            ApprovedDiscourseRelationIR::Statement,
                            ApprovedOperationIR::Assert,
                            vec![claim(
                                "P_BOOLEAN_SPEECH_ACT",
                                subject_id,
                                subject_label,
                                relation,
                                ApprovedOpenValueIR::Boolean(asserted),
                            )],
                            ApprovedVerbosityIR::Short,
                        );
                        approved.speech_act = speech_act;
                        approved.discourse_relation = discourse;
                        approved.operation = operation;
                        approved.style.register = register;
                        approved.semantic_sha256 = compositional_response_sha256(&approved);
                        assert!(approved.validate());

                        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                            .unwrap_or_else(|error| {
                                panic!(
                                    "{speech_act:?}/{relation:?}/{asserted}/{register:?}: {error}"
                                )
                            });
                        assert!(rendered.validate(&approved));
                        assert!(!rendered.markdown.contains("맞음"));
                        assert!(!rendered.markdown.contains("아님"));
                        assert!(!rendered.markdown.contains("여부는"));
                    }
                }
            }
        }
    }

    #[test]
    fn boolean_speech_act_registry_has_natural_formal_surfaces() {
        let cases = [
            (
                ApprovedSpeechActIR::Query,
                ApprovedRelationTypeIR::Cancelled,
                true,
                "회의는 취소됐습니까?",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedRelationTypeIR::RoomAvailable,
                true,
                "회의실을 사용할 수 있습니까?",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedRelationTypeIR::Registration,
                true,
                "워크숍은 등록됐습니까?",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedRelationTypeIR::Entry,
                true,
                "워크숍에 입장할 수 있습니까?",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedRelationTypeIR::Confirmed,
                true,
                "회의는 확정됐습니까?",
            ),
            (
                ApprovedSpeechActIR::Query,
                ApprovedRelationTypeIR::Approved,
                true,
                "교육 예산은 승인됐습니까?",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedRelationTypeIR::Cancelled,
                true,
                "회의를 취소해 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedRelationTypeIR::Registration,
                true,
                "워크숍을 등록해 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedRelationTypeIR::Confirmed,
                false,
                "회의를 확정하지 말아 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Request,
                ApprovedRelationTypeIR::Approved,
                true,
                "교육 예산을 승인해 주십시오.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedRelationTypeIR::Cancelled,
                false,
                "회의를 취소하지 않겠습니다.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedRelationTypeIR::Registration,
                true,
                "워크숍을 등록하겠습니다.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedRelationTypeIR::Confirmed,
                true,
                "회의를 확정하겠습니다.",
            ),
            (
                ApprovedSpeechActIR::Promise,
                ApprovedRelationTypeIR::Approved,
                true,
                "교육 예산을 승인하겠습니다.",
            ),
        ];

        for (speech_act, relation, asserted, expected) in cases {
            let (subject_id, subject_label) = match relation {
                ApprovedRelationTypeIR::RoomAvailable => ("meeting_room", "회의실"),
                ApprovedRelationTypeIR::Registration | ApprovedRelationTypeIR::Entry => {
                    ("workshop", "워크숍")
                }
                ApprovedRelationTypeIR::Approved => ("budget", "교육 예산"),
                _ => ("meeting", "회의"),
            };
            let (discourse, operation) = match speech_act {
                ApprovedSpeechActIR::Query => (
                    ApprovedDiscourseRelationIR::Inquiry,
                    ApprovedOperationIR::Query,
                ),
                ApprovedSpeechActIR::Request => (
                    ApprovedDiscourseRelationIR::Directive,
                    ApprovedOperationIR::Request,
                ),
                ApprovedSpeechActIR::Promise => (
                    ApprovedDiscourseRelationIR::Commitment,
                    ApprovedOperationIR::Promise,
                ),
                _ => unreachable!(),
            };
            let mut approved = response(
                ApprovedDiscourseRelationIR::Statement,
                ApprovedOperationIR::Assert,
                vec![claim(
                    "P_BOOLEAN_SURFACE",
                    subject_id,
                    subject_label,
                    relation,
                    ApprovedOpenValueIR::Boolean(asserted),
                )],
                ApprovedVerbosityIR::Short,
            );
            approved.speech_act = speech_act;
            approved.discourse_relation = discourse;
            approved.operation = operation;
            approved.semantic_sha256 = compositional_response_sha256(&approved);
            let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
                .expect("boolean speech-act surface should roundtrip");
            assert_eq!(rendered.markdown, expected);
        }
    }

    #[test]
    fn progress_status_avoids_repeating_the_process_noun_and_roundtrips() {
        let approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_REVIEW_PROGRESS",
                "screen_design_review",
                "화면 설계 검토",
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "under_review".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "검토 중".into(),
                }),
            )],
            ApprovedVerbosityIR::Short,
        );
        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("progress status should roundtrip");
        assert_eq!(rendered.markdown, "화면 설계는 검토 중입니다.");
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn typed_action_event_binding_realizes_a_verified_action_without_nominal_fallback() {
        let (claims, event_realization) = ApprovedActionEventFrameIR {
            action_proposition_id: "P_INCIDENT_ACTION".into(),
            action_subject: ApprovedLexicalNodeIR {
                node_id: "payment_incident".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "결제 장애".into(),
            },
            event: ApprovedLexicalNodeIR {
                node_id: "coupon_deactivation".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "할인 쿠폰 배포 비활성화".into(),
            },
            class: ApprovedEventRealizationClassIR::StateChange,
            predicate_sense: ApprovedEventPredicateSenseIR::Deactivate,
            phase: ApprovedEventPhaseIR::Planned,
            perspective: ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            },
            action_modality: ApprovedModalityIR::Asserted,
            arguments: vec![
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_ACTION_AGENT".into(),
                    relation: ApprovedRelationTypeIR::Agent,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "operations_team".into(),
                        semantic_type: ApprovedSemanticTypeIR::Organization,
                        canonical_lexical_label: "운영팀".into(),
                    }),
                },
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_ACTION_THEME".into(),
                    relation: ApprovedRelationTypeIR::Theme,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "coupon_distribution".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: "할인 쿠폰 배포".into(),
                    }),
                },
            ],
        }
        .materialize()
        .expect("complete source action event");
        let mut approved = response(
            ApprovedDiscourseRelationIR::Cause,
            ApprovedOperationIR::Explain,
            claims,
            ApprovedVerbosityIR::Short,
        );
        approved.event_realizations.push(event_realization);
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        assert!(approved.validate());

        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("typed action event should roundtrip");
        assert!(
            rendered
                .markdown
                .contains("운영팀이 할인 쿠폰 배포를 비활성화할 예정입니다.")
        );
        assert!(rendered.validate(&approved));
        assert_eq!(rendered.semantic_interpretation.recovered_claim_ids.len(), 3);
    }

    #[test]
    fn commissive_action_event_routes_promise_through_typed_predicate() {
        let (claims, event_realization) = ApprovedActionEventFrameIR {
            action_proposition_id: "P_CALIBRATION_ACTION".into(),
            action_subject: ApprovedLexicalNodeIR {
                node_id: "projector_calibration".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "천체투영기 보정".into(),
            },
            event: ApprovedLexicalNodeIR {
                node_id: "optical_axis_alignment".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "광축 정렬".into(),
            },
            class: ApprovedEventRealizationClassIR::StateChange,
            predicate_sense: ApprovedEventPredicateSenseIR::Align,
            phase: ApprovedEventPhaseIR::Planned,
            perspective: ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Passive,
                focus: ApprovedEventFocusIR::Theme,
            },
            action_modality: ApprovedModalityIR::Commissive,
            arguments: vec![ApprovedActionEventArgumentIR {
                proposition_id: "P_CALIBRATION_THEME".into(),
                relation: ApprovedRelationTypeIR::Theme,
                value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "projector_axis".into(),
                    semantic_type: ApprovedSemanticTypeIR::Concept,
                    canonical_lexical_label: "천체투영기 광축".into(),
                }),
            }],
        }
        .materialize()
        .expect("complete source commitment event");
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            claims,
            ApprovedVerbosityIR::Short,
        );
        approved.speech_act = ApprovedSpeechActIR::Promise;
        approved.operation = ApprovedOperationIR::Promise;
        approved.discourse_relation = ApprovedDiscourseRelationIR::Commitment;
        approved.style.register = LanguageRegisterIR::Neutral;
        approved.event_realizations.push(event_realization);
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        assert!(approved.validate());

        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("commissive action should roundtrip");
        assert_eq!(rendered.markdown, "1. 천체투영기 광축을 정렬할게요.");
        assert!(rendered.validate(&approved));
        assert_eq!(rendered.unsupported_claims, 0);
    }

    #[test]
    fn directive_procedure_order_is_visible_and_roundtrips_without_a_relation_section() {
        let procedure = ApprovedLexicalNodeIR {
            node_id: "freezer_relocation_procedure".into(),
            semantic_type: ApprovedSemanticTypeIR::Event,
            canonical_lexical_label: "식물표본 냉동고 이전".into(),
        };
        let frost_removal = ApprovedActionEventFrameIR {
            action_proposition_id: "P_FREEZER_FROST_REMOVAL".into(),
            action_subject: procedure.clone(),
            event: ApprovedLexicalNodeIR {
                node_id: "freezer_frost_removal".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "냉동고 성에 제거".into(),
            },
            class: ApprovedEventRealizationClassIR::StateChange,
            predicate_sense: ApprovedEventPredicateSenseIR::Remove,
            phase: ApprovedEventPhaseIR::Planned,
            perspective: ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Passive,
                focus: ApprovedEventFocusIR::Theme,
            },
            action_modality: ApprovedModalityIR::Directive,
            arguments: vec![ApprovedActionEventArgumentIR {
                proposition_id: "P_FREEZER_FROST_THEME".into(),
                relation: ApprovedRelationTypeIR::Theme,
                value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "freezer_frost".into(),
                    semantic_type: ApprovedSemanticTypeIR::Concept,
                    canonical_lexical_label: "냉동고 성에".into(),
                }),
            }],
        };
        let relocation = ApprovedActionEventFrameIR {
            action_proposition_id: "P_FREEZER_RELOCATION".into(),
            action_subject: procedure.clone(),
            event: ApprovedLexicalNodeIR {
                node_id: "herbarium_freezer_relocation".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "식물표본 냉동고 이전".into(),
            },
            class: ApprovedEventRealizationClassIR::Motion,
            predicate_sense: ApprovedEventPredicateSenseIR::Move,
            phase: ApprovedEventPhaseIR::Planned,
            perspective: ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Passive,
                focus: ApprovedEventFocusIR::Theme,
            },
            action_modality: ApprovedModalityIR::Directive,
            arguments: vec![
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_FREEZER_RELOCATION_THEME".into(),
                    relation: ApprovedRelationTypeIR::Theme,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "herbarium_freezer".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: "식물표본 냉동고".into(),
                    }),
                },
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_FREEZER_RELOCATION_DESTINATION".into(),
                    relation: ApprovedRelationTypeIR::Destination,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "specimen_quarantine_room".into(),
                        semantic_type: ApprovedSemanticTypeIR::Location,
                        canonical_lexical_label: "표본 검역실".into(),
                    }),
                },
            ],
        };
        let (mut claims, frost_realization) = frost_removal.materialize().unwrap();
        let (relocation_claims, relocation_realization) = relocation.materialize().unwrap();
        claims.extend(relocation_claims);
        claims.push(ApprovedCompositionalClaimIR {
            proposition_id: "P_FREEZER_FROST_BEFORE_RELOCATION".into(),
            subject: frost_removal.event.clone(),
            relation: ApprovedRelationTypeIR::EarlierThan,
            value: ApprovedOpenValueIR::Lexical(relocation.event.clone()),
            polarity: true,
            modality: ApprovedModalityIR::Directive,
    status_frame: None,
});
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            vec![claim(
                "P_PROCEDURE_SEED",
                "freezer_relocation_procedure",
                "식물표본 냉동고 이전",
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "planned".into(),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: "계획".into(),
                }),
            )],
            ApprovedVerbosityIR::Explanatory,
        );
        approved.speech_act = ApprovedSpeechActIR::Request;
        approved.claims = claims;
        approved.clause_plan = approved
            .claims
            .iter()
            .enumerate()
            .map(|(index, claim)| ApprovedClauseUnitIR {
                unit_index: index,
                role: "ASSERT".into(),
                proposition_ids: vec![claim.proposition_id.clone()],
                predecessor_indices: (index > 0).then(|| index - 1).into_iter().collect(),
            })
            .collect();
        approved.event_realizations = vec![frost_realization, relocation_realization];
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        assert!(approved.validate());

        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("ordered directive procedure should realize");
        assert!(rendered.markdown.contains("먼저 냉동고 성에를 제거해야 합니다."));
        assert!(rendered
            .markdown
            .contains("그다음 식물표본 냉동고를 표본 검역실로 이동해야 합니다."));
        assert!(!rendered.markdown.contains("선행 관계"));
        assert!(rendered.validate(&approved));
        assert!(rendered
            .semantic_interpretation
            .recovered_claim_ids
            .contains(&"P_FREEZER_FROST_BEFORE_RELOCATION".to_string()));
    }

    #[test]
    fn long_document_keeps_a_typed_action_and_its_event_arguments_in_one_section() {
        let (mut claims, event_realization) = ApprovedActionEventFrameIR {
            action_proposition_id: "P_LONG_ACTION".into(),
            action_subject: ApprovedLexicalNodeIR {
                node_id: "incident_response".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "장애 대응".into(),
            },
            event: ApprovedLexicalNodeIR {
                node_id: "traffic_reroute".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "통신 트래픽 우회".into(),
            },
            class: ApprovedEventRealizationClassIR::Motion,
            predicate_sense: ApprovedEventPredicateSenseIR::Move,
            phase: ApprovedEventPhaseIR::Ongoing,
            perspective: ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            },
            action_modality: ApprovedModalityIR::Asserted,
            arguments: vec![
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_LONG_ACTION_AGENT".into(),
                    relation: ApprovedRelationTypeIR::Agent,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "network_operations".into(),
                        semantic_type: ApprovedSemanticTypeIR::Organization,
                        canonical_lexical_label: "망 관제팀".into(),
                    }),
                },
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_LONG_ACTION_THEME".into(),
                    relation: ApprovedRelationTypeIR::Theme,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "island_traffic".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: "도서 지역 통신 트래픽".into(),
                    }),
                },
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_LONG_ACTION_DESTINATION".into(),
                    relation: ApprovedRelationTypeIR::Destination,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "eastern_landing_station".into(),
                        semantic_type: ApprovedSemanticTypeIR::Location,
                        canonical_lexical_label: "동부 육양국".into(),
                    }),
                },
            ],
        }
        .materialize()
        .expect("complete long-document Action→Event source frame");
        for index in 0..17 {
            claims.push(claim(
                &format!("P_LONG_STATUS_{index:02}"),
                &format!("long_status_{index:02}"),
                &format!("상태 항목 {}", index + 1),
                ApprovedRelationTypeIR::Status,
                ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: format!("long_state_{index:02}"),
                    semantic_type: ApprovedSemanticTypeIR::State,
                    canonical_lexical_label: format!("확인 항목 {}", index + 1),
                }),
            ));
        }
        let mut approved = response(
            ApprovedDiscourseRelationIR::Cause,
            ApprovedOperationIR::Explain,
            claims,
            ApprovedVerbosityIR::Explanatory,
        );
        approved.event_realizations.push(event_realization);
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        assert!(approved.validate());

        let sections = plan_long_document_sections(&approved);
        let action_section = sections
            .iter()
            .find(|section| section.claim_ids.iter().any(|id| id == "P_LONG_ACTION"))
            .expect("typed action appears in a long-document section");
        assert!(action_section
            .claim_ids
            .iter()
            .any(|id| id == "P_LONG_ACTION_AGENT"));
        assert!(action_section
            .claim_ids
            .iter()
            .any(|id| id == "P_LONG_ACTION_THEME"));
        assert!(action_section
            .claim_ids
            .iter()
            .any(|id| id == "P_LONG_ACTION_DESTINATION"));

        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("long typed action must render and inverse exactly");
        assert!(rendered
            .markdown
            .contains("망 관제팀이 도서 지역 통신 트래픽을 동부 육양국으로 옮기고 있습니다."));
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn planned_attachment_action_keeps_its_typed_target_in_the_event_clause() {
        let (claims, event_realization) = ApprovedActionEventFrameIR {
            action_proposition_id: "P_PREVENTION_ACTION".into(),
            action_subject: ApprovedLexicalNodeIR {
                node_id: "recurrence_prevention".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "재발 방지안".into(),
            },
            event: ApprovedLexicalNodeIR {
                node_id: "drain_warning_installation".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "재발 방지 설비 설치".into(),
            },
            class: ApprovedEventRealizationClassIR::StateChange,
            predicate_sense: ApprovedEventPredicateSenseIR::Attach,
            phase: ApprovedEventPhaseIR::Planned,
            perspective: ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            },
            action_modality: ApprovedModalityIR::Asserted,
            arguments: vec![
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_PREVENTION_AGENT".into(),
                    relation: ApprovedRelationTypeIR::Agent,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "preservation_lead".into(),
                        semantic_type: ApprovedSemanticTypeIR::Person,
                        canonical_lexical_label: "보존시설 책임자".into(),
                    }),
                },
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_PREVENTION_THEME".into(),
                    relation: ApprovedRelationTypeIR::Theme,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "drain_warning".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: "이중 거름망과 수위 경보".into(),
                    }),
                },
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_PREVENTION_TARGET".into(),
                    relation: ApprovedRelationTypeIR::Target,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "drain_outlets".into(),
                        semantic_type: ApprovedSemanticTypeIR::Location,
                        canonical_lexical_label: "배수구".into(),
                    }),
                },
            ],
        }
        .materialize()
        .expect("complete attachment source event");
        let mut approved = response(
            ApprovedDiscourseRelationIR::Statement,
            ApprovedOperationIR::Assert,
            claims,
            ApprovedVerbosityIR::Short,
        );
        approved.event_realizations.push(event_realization);
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        assert!(approved.validate());

        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("planned attachment should be realizable");
        assert!(rendered.markdown.contains(
            "보존시설 책임자가 이중 거름망과 수위 경보를 배수구에 부착할 예정입니다."
        ));
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn long_document_keeps_a_final_typed_attachment_bundle_intact() {
        let (mut action_claims, event_realization) = ApprovedActionEventFrameIR {
            action_proposition_id: "P_FINAL_ACTION".into(),
            action_subject: ApprovedLexicalNodeIR {
                node_id: "final_prevention".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "최종 재발 방지안".into(),
            },
            event: ApprovedLexicalNodeIR {
                node_id: "final_attachment".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "배수구 보강 설치".into(),
            },
            class: ApprovedEventRealizationClassIR::StateChange,
            predicate_sense: ApprovedEventPredicateSenseIR::Attach,
            phase: ApprovedEventPhaseIR::Planned,
            perspective: ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Active,
                focus: ApprovedEventFocusIR::Agent,
            },
            action_modality: ApprovedModalityIR::Asserted,
            arguments: vec![
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_FINAL_AGENT".into(),
                    relation: ApprovedRelationTypeIR::Agent,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "final_lead".into(),
                        semantic_type: ApprovedSemanticTypeIR::Person,
                        canonical_lexical_label: "시설 책임자".into(),
                    }),
                },
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_FINAL_THEME".into(),
                    relation: ApprovedRelationTypeIR::Theme,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "final_mesh".into(),
                        semantic_type: ApprovedSemanticTypeIR::Concept,
                        canonical_lexical_label: "이중 거름망".into(),
                    }),
                },
                ApprovedActionEventArgumentIR {
                    proposition_id: "P_FINAL_TARGET".into(),
                    relation: ApprovedRelationTypeIR::Target,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: "final_drain".into(),
                        semantic_type: ApprovedSemanticTypeIR::Location,
                        canonical_lexical_label: "배수구".into(),
                    }),
                },
            ],
        }
        .materialize()
        .expect("complete final attachment source event");
        let mut claims = (0..20)
            .map(|index| {
                claim(
                    &format!("P_PREFIX_STATUS_{index:02}"),
                    &format!("prefix_status_{index:02}"),
                    &format!("선행 상태 {}", index + 1),
                    ApprovedRelationTypeIR::Status,
                    ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: format!("prefix_value_{index:02}"),
                        semantic_type: ApprovedSemanticTypeIR::State,
                        canonical_lexical_label: format!("확인값 {}", index + 1),
                    }),
                )
            })
            .collect::<Vec<_>>();
        let (mut preceding_claims, preceding_realization) = ApprovedActionEventFrameIR {
            action_proposition_id: "P_PRECEDING_ACTION".into(),
            action_subject: ApprovedLexicalNodeIR {
                node_id: "preceding_review".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "선행 원인 점검".into(),
            },
            event: ApprovedLexicalNodeIR {
                node_id: "preceding_inspection".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "선행 점검".into(),
            },
            class: ApprovedEventRealizationClassIR::Inspection,
            predicate_sense: ApprovedEventPredicateSenseIR::Inspect,
            phase: ApprovedEventPhaseIR::Planned,
            perspective: ApprovedEventPerspectiveIR {
                voice: ApprovedEventVoiceIR::Passive,
                focus: ApprovedEventFocusIR::Theme,
            },
            action_modality: ApprovedModalityIR::Asserted,
            arguments: vec![ApprovedActionEventArgumentIR {
                proposition_id: "P_PRECEDING_THEME".into(),
                relation: ApprovedRelationTypeIR::Theme,
                value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "preceding_subject".into(),
                    semantic_type: ApprovedSemanticTypeIR::Concept,
                    canonical_lexical_label: "배수 설계".into(),
                }),
            }],
        }
        .materialize()
        .expect("complete preceding inspection source event");
        claims.append(&mut preceding_claims);
        claims.push(claim(
            "P_FINAL_OWNER",
            "final_owner_record",
            "최종 조치 담당",
            ApprovedRelationTypeIR::Owner,
            ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                node_id: "final_lead".into(),
                semantic_type: ApprovedSemanticTypeIR::Person,
                canonical_lexical_label: "시설 책임자".into(),
            }),
        ));
        claims.append(&mut action_claims);
        let mut approved = response(
            ApprovedDiscourseRelationIR::Cause,
            ApprovedOperationIR::Explain,
            claims,
            ApprovedVerbosityIR::Explanatory,
        );
        approved.event_realizations.push(preceding_realization);
        approved.event_realizations.push(event_realization);
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        assert!(approved.validate());

        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("final action frame must survive long planning");
        assert!(rendered.markdown.contains(
            "시설 책임자가 이중 거름망을 배수구에 부착할 예정입니다."
        ));
        assert!(rendered.validate(&approved));
    }

    #[test]
    fn acknowledgement_is_explicit_and_does_not_replace_the_approved_claim() {
        let mut approved = response(
            ApprovedDiscourseRelationIR::Confirmation,
            ApprovedOperationIR::Confirm,
            vec![claim(
                "P_REMOTE_WORK_DATE",
                "remote_work",
                "재택근무",
                ApprovedRelationTypeIR::Date,
                ApprovedOpenValueIR::Date {
                    year: 2026,
                    month: 9,
                    day: 23,
                },
            )],
            ApprovedVerbosityIR::Short,
        );
        approved.speech_act = ApprovedSpeechActIR::Acknowledge;
        approved.style.register = LanguageRegisterIR::Neutral;
        approved.semantic_sha256 = compositional_response_sha256(&approved);
        assert!(approved.validate());

        let rendered = realize_document_response(&approved, LanguageCodeIR::Korean)
            .expect("acknowledgement should preserve and recover the approved claim");
        assert_eq!(
            rendered.markdown,
            "확인했어요.\n\n재택근무 날짜는 2026년 9월 23일이에요."
        );
        assert!(rendered.validate(&approved));
        assert_eq!(
            rendered.semantic_interpretation.recovered_claim_ids.len(),
            1
        );
    }
}
