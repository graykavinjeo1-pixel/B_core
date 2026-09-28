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
    ApprovedDiscourseRelationIR, ApprovedEventFocusIR, ApprovedEventInformationRoleIR,
    ApprovedEventInformationStructureIR, ApprovedEventPerspectiveIR, ApprovedEventPhaseIR,
    ApprovedEventPredicateSenseIR, ApprovedEventRealizationClassIR, ApprovedEventVoiceIR,
    ApprovedLexicalNodeIR, ApprovedModalityIR, ApprovedOpenValueIR, ApprovedOperationIR,
    ApprovedRelationTypeIR, ApprovedSemanticTypeIR, ApprovedVerbosityIR,
};
use crate::language_knowledge::{LanguageCodeIR, LanguageRegisterIR};

pub const DOCUMENT_RESPONSE_PLAN_SCHEMA: &str = "B_CORE_DOCUMENT_RESPONSE_PLAN_IR_9";
pub const DOCUMENT_RESPONSE_OUTPUT_SCHEMA: &str = "B_CORE_DOCUMENT_RESPONSE_OUTPUT_IR_11";
pub const DOCUMENT_SURFACE_STRUCTURE_SCHEMA: &str = "B_CORE_DOCUMENT_SURFACE_STRUCTURE_IR_1";
pub const DOCUMENT_SEMANTIC_INTERPRETATION_SCHEMA: &str =
    "B_CORE_DOCUMENT_SEMANTIC_INTERPRETATION_IR_9";

const MAX_BLOCKS: usize = 32;
const MAX_SECTIONS: usize = 8;
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
        let approved_order = response
            .claims
            .iter()
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
        .map(|claim| claim.proposition_id.clone())
        .collect::<Vec<_>>();
    let section_specs = plan_document_sections(response);
    let mut blocks = Vec::new();
    let mut sections = Vec::new();
    let explanatory = response.style.verbosity == ApprovedVerbosityIR::Explanatory
        || response.speech_act == crate::approved_response::ApprovedSpeechActIR::Explain
        || response.claims.len() > 2;
    if explanatory {
        push_block(
            &mut blocks,
            DocumentResponseBlockKindIR::Lead,
            DocumentResponseRoleIR::Orientation,
            Vec::new(),
        );
        push_block(
            &mut blocks,
            DocumentResponseBlockKindIR::SectionHeading,
            role_for(response.discourse_relation),
            Vec::new(),
        );
    }
    let multi_section = section_specs.len() > 1;
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
        let content_kind = if response.discourse_relation == ApprovedDiscourseRelationIR::Comparison
            || chartable
        {
            DocumentResponseBlockKindIR::Table
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
        .ok_or_else(|| "DOCUMENT_RESPONSE_OUTPUT_VALIDATION_FAILED".into())
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
                if node.surface == lead_surface(output_language)
                    || node.surface == closing_surface(output_language)
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
                modality: ApprovedModalityIR::Asserted,
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
    interpretation
        .validate(response)
        .then_some(interpretation)
        .ok_or_else(|| "DOCUMENT_SEMANTIC_INTERPRETATION_FAILED".into())
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

const ALL_RELATIONS: [ApprovedRelationTypeIR; 24] = [
    ApprovedRelationTypeIR::Status,
    ApprovedRelationTypeIR::Time,
    ApprovedRelationTypeIR::Cancelled,
    ApprovedRelationTypeIR::EarlierThan,
    ApprovedRelationTypeIR::RoomAvailable,
    ApprovedRelationTypeIR::Location,
    ApprovedRelationTypeIR::Registration,
    ApprovedRelationTypeIR::Entry,
    ApprovedRelationTypeIR::Confirmed,
    ApprovedRelationTypeIR::Capacity,
    ApprovedRelationTypeIR::Duration,
    ApprovedRelationTypeIR::Name,
    ApprovedRelationTypeIR::Count,
    ApprovedRelationTypeIR::Date,
    ApprovedRelationTypeIR::Quantity,
    ApprovedRelationTypeIR::Agent,
    ApprovedRelationTypeIR::Patient,
    ApprovedRelationTypeIR::Theme,
    ApprovedRelationTypeIR::Source,
    ApprovedRelationTypeIR::Destination,
    ApprovedRelationTypeIR::Instrument,
    ApprovedRelationTypeIR::Manner,
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
    const INVERSES: [(&str, [&str; 2]); 8] = [
        ("되지 않았고", ["되지 않았습니다", "되지 않았어요"]),
        ("할 수 없고", ["할 수 없습니다", "할 수 없어요"]),
        ("할 수 있고", ["할 수 있습니다", "할 수 있어요"]),
        ("변경됐고", ["변경됐습니다", "변경됐어요"]),
        ("아니고", ["아닙니다", "아니에요"]),
        ("됐고", ["됐습니다", "됐어요"]),
        ("이고", ["입니다", "이에요"]),
        ("맞고", ["맞습니다", "맞아요"]),
    ];
    for delimiter in delimiters_outside_text(surface, ", ") {
        let left = &surface[..delimiter];
        for (coordinated, finite_endings) in INVERSES {
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
        if character != '.' {
            continue;
        }
        *attempts += 1;
        let end = offset + character.len_utf8();
        let sentence = remaining[..end].trim();
        let context_subject = current.last().map(|(_, fact)| &fact.subject);
        let candidates = parse_one_semantic_sentence(sentence, response, language, context_subject);
        for candidate in candidates {
            current.push((sentence.to_string(), candidate));
            parse_sentence_suffix(
                &remaining[end..],
                response,
                language,
                attempts,
                current,
                solutions,
            );
            current.pop();
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
) -> Vec<ParsedSurfaceClaim> {
    let Some(body) = sentence.strip_suffix('.') else {
        return Vec::new();
    };
    let mut parsed = match language {
        LanguageCodeIR::Korean => parse_korean_semantic_sentence(body, response),
        LanguageCodeIR::English => parse_english_semantic_sentence(body, response),
        _ => Vec::new(),
    };
    if let Some(rhetorical_body) = strip_allowed_discourse_prefix(body, response, language) {
        parsed.extend(match language {
            LanguageCodeIR::Korean => parse_korean_semantic_sentence(&rhetorical_body, response),
            LanguageCodeIR::English => parse_english_semantic_sentence(&rhetorical_body, response),
            _ => Vec::new(),
        });
    }
    if let Some(subject) = context_subject {
        parsed.extend(parse_contextual_followup(body, response, language, subject));
    }
    parsed.dedup();
    parsed
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
                    .then(|| body.strip_prefix("정정하면, "))
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
            for ending in [" 변경됐습니다", " 변경됐어요"] {
                if let Some(value_with_particle) = predicate.strip_suffix(ending) {
                    for value in
                        parse_korean_particle_value(value_with_particle, "으로", "로", response)
                    {
                        push_parsed(&mut parsed, subject.clone(), relation, value, true);
                    }
                }
            }
            for ending in [" 아닙니다", " 아니에요"] {
                if let Some(value_with_particle) = predicate.strip_suffix(ending) {
                    for value in
                        parse_korean_particle_value(value_with_particle, "이", "가", response)
                    {
                        push_parsed(&mut parsed, subject.clone(), relation, value, false);
                    }
                }
            }
            for ending in ["입니다", "이에요", "예요"] {
                if let Some(value_surface) = predicate.strip_suffix(ending) {
                    for value in parse_open_values(value_surface, response, LanguageCodeIR::Korean)
                    {
                        push_parsed(&mut parsed, subject.clone(), relation, value, true);
                    }
                }
            }
        }
    }
    parsed
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
            false,
            "사용할 수 없습니다",
        ),
        (
            ApprovedRelationTypeIR::RoomAvailable,
            false,
            "사용할 수 없어요",
        ),
        (ApprovedRelationTypeIR::Registration, true, "등록됐습니다"),
        (ApprovedRelationTypeIR::Registration, true, "등록됐어요"),
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
        (ApprovedRelationTypeIR::Entry, true, "입장할 수 있습니다"),
        (ApprovedRelationTypeIR::Entry, true, "입장할 수 있어요"),
        (ApprovedRelationTypeIR::Entry, false, "입장할 수 없습니다"),
        (ApprovedRelationTypeIR::Entry, false, "입장할 수 없어요"),
        (ApprovedRelationTypeIR::Confirmed, true, "확정됐습니다"),
        (ApprovedRelationTypeIR::Confirmed, true, "확정됐어요"),
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
    ]
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
    ]
}

fn korean_truth_predicate(predicate: &str) -> Option<bool> {
    match predicate {
        "맞습니다" | "맞아요" => Some(true),
        "아닙니다" | "아니에요" => Some(false),
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
                    if object_particle(particle_basis(&value, value_surface), consonant, vowel).0
                        != particle
                    {
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
        if let ApprovedOpenValueIR::Lexical(node) = &claim.value {
            if node.canonical_lexical_label == surface
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
    response
        .claims
        .iter()
        .enumerate()
        .map(|(index, claim)| {
            let rhetorical_move = match response.discourse_relation {
                ApprovedDiscourseRelationIR::Correction => DocumentRhetoricalMoveIR::Correction,
                ApprovedDiscourseRelationIR::Cause if response.claims.len() > 1 => {
                    if index == last_index {
                        DocumentRhetoricalMoveIR::Conclusion
                    } else {
                        DocumentRhetoricalMoveIR::Cause
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
                    if index == last_index {
                        DocumentRhetoricalMoveIR::Conclusion
                    } else {
                        DocumentRhetoricalMoveIR::Support
                    }
                }
                _ => DocumentRhetoricalMoveIR::Assertion,
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
            let first_move = rhetorical_units
                .iter()
                .find(|unit| unit.proposition_id == first.proposition_id)
                .map(|unit| unit.rhetorical_move);
            let mut end = start + 1;
            let mut last_order = event_argument_order(first.relation).expect("typed argument");
            while end < claims.len() && end - start < MAX_EVENT_ARGUMENTS {
                let candidate = claims[end];
                let candidate_move = rhetorical_units
                    .iter()
                    .find(|unit| unit.proposition_id == candidate.proposition_id)
                    .map(|unit| unit.rhetorical_move);
                let Some(order) = event_argument_order(candidate.relation) else {
                    break;
                };
                if candidate.subject != first.subject
                    || candidate_move != first_move
                    || !is_typed_event_argument(candidate, response)
                    || order <= last_order
                {
                    break;
                }
                last_order = order;
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
                            candidate.subject == first.subject && candidate_move == first_move
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
    predicates
}

fn is_typed_event_argument(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> bool {
    if claim.subject.semantic_type != ApprovedSemanticTypeIR::Event
        || approved_event_predicate(response, &claim.subject.node_id).is_none()
        || !claim.polarity
        || !matches!(
            response.operation,
            ApprovedOperationIR::Assert
                | ApprovedOperationIR::Explain
                | ApprovedOperationIR::Recall
                | ApprovedOperationIR::Confirm
        )
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
            ApprovedOpenValueIR::Clock { .. }
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
    (scheduled_argument
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
        ApprovedRelationTypeIR::InitialState => Some(5),
        ApprovedRelationTypeIR::ResultState => Some(6),
        ApprovedRelationTypeIR::Date => Some(7),
        ApprovedRelationTypeIR::Time => Some(8),
        ApprovedRelationTypeIR::Location => Some(9),
        ApprovedRelationTypeIR::Instrument => Some(10),
        ApprovedRelationTypeIR::Manner => Some(11),
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

    let mut sections = Vec::<PlannedDocumentSection>::new();
    for claim in &response.claims {
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

    while sections.len() > MAX_SECTIONS {
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
    sections
}

fn role_for_claim(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> DocumentResponseRoleIR {
    if is_typed_event_argument(claim, response)
        && response.claims.iter().any(|candidate| {
            candidate.proposition_id != claim.proposition_id
                && candidate.subject == claim.subject
                && is_typed_event_argument(candidate, response)
                && candidate.relation != claim.relation
        })
    {
        return match approved_event_predicate(response, &claim.subject.node_id) {
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
        | ApprovedRelationTypeIR::Confirmed => DocumentResponseRoleIR::Status,
        ApprovedRelationTypeIR::Time
        | ApprovedRelationTypeIR::Date
        | ApprovedRelationTypeIR::Duration
        | ApprovedRelationTypeIR::EarlierThan => DocumentResponseRoleIR::Timeline,
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
        ApprovedDiscourseRelationIR::Temporal => DocumentResponseRoleIR::Timeline,
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
            DocumentResponseBlockKindIR::Lead => lead_surface(plan.output_language),
            DocumentResponseBlockKindIR::SectionHeading => {
                format!("## {}", heading_surface(block.role, plan.output_language))
            }
            DocumentResponseBlockKindIR::SubsectionHeading => {
                format!("### {}", heading_surface(block.role, plan.output_language))
            }
            DocumentResponseBlockKindIR::Paragraph => {
                realize_claim_sequence(&claims, plan, response, plan.output_language)
            }
            DocumentResponseBlockKindIR::OrderedList => contiguous_subject_groups(&claims)
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
            DocumentResponseBlockKindIR::Closing => closing_surface(plan.output_language),
        };
        if !surface.is_empty() {
            rendered.push(surface);
        }
    }
    rendered.join("\n\n")
}

fn lead_surface(language: LanguageCodeIR) -> String {
    match language {
        LanguageCodeIR::Korean => "확인된 내용을 정리하면 다음과 같습니다.".into(),
        _ => "The confirmed points are as follows.".into(),
    }
}

fn closing_surface(language: LanguageCodeIR) -> String {
    match language {
        LanguageCodeIR::Korean => "확인된 근거 범위 안에서 정리했습니다.".into(),
        _ => "This response stays within the confirmed evidence.".into(),
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

fn contiguous_subject_groups<'a>(
    claims: &[&'a ApprovedCompositionalClaimIR],
) -> Vec<Vec<&'a ApprovedCompositionalClaimIR>> {
    let mut groups = Vec::<Vec<&ApprovedCompositionalClaimIR>>::new();
    for claim in claims {
        if let Some(group) = groups.last_mut().filter(|group| {
            group
                .last()
                .is_some_and(|previous| previous.subject == claim.subject)
        }) {
            group.push(*claim);
        } else {
            groups.push(vec![*claim]);
        }
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
    let mut claim_index = 0usize;
    while claim_index < claims.len() {
        let claim = claims[claim_index];
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
    rendered
        .into_iter()
        .map(|(_, surface)| surface)
        .collect::<Vec<_>>()
        .join(" ")
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
                    | DocumentEventPredicateKindIR::StateChange => return None,
                };
                circumstances.push(format!("{location}{particle}"));
            }
            let topic = object_particle(subject, "은", "는").0;
            let formal = response.style.register == LanguageRegisterIR::Formal;
            let ending = korean_event_predicate(predicate.kind, predicate.phase, formal)?;
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
    let initial_state = displayed(DocumentEventArgumentRoleIR::InitialState);
    let result_state = displayed(DocumentEventArgumentRoleIR::ResultState);
    let date = displayed(DocumentEventArgumentRoleIR::Date);
    let time = displayed(DocumentEventArgumentRoleIR::Time);
    let location = displayed(DocumentEventArgumentRoleIR::Location);
    let instrument = displayed(DocumentEventArgumentRoleIR::Instrument);
    let manner = displayed(DocumentEventArgumentRoleIR::Manner);

    match language {
        LanguageCodeIR::Korean => {
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
                Some(ApprovedEventInformationRoleIR::Source) => {
                    phrases.push(format!("{}에서는", source.as_deref()?));
                }
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
            if information.topic != Some(ApprovedEventInformationRoleIR::Patient) {
                if let Some(patient) = patient {
                    phrases.push(format!("{patient}에게"));
                }
            }
            if information.topic != Some(ApprovedEventInformationRoleIR::Theme)
                && information.voice == ApprovedEventVoiceIR::Active
            {
                phrases.push(with_particle(&theme, "을", "를"));
            }
            if information.topic != Some(ApprovedEventInformationRoleIR::Source) {
                if let Some(source) = source {
                    phrases.push(format!("{source}에서"));
                }
            }
            if information.topic != Some(ApprovedEventInformationRoleIR::Destination) {
                if let Some(destination) = destination {
                    phrases.push(with_directional_particle(&destination));
                }
            }
            if let Some(initial_state) = initial_state {
                phrases.push(format!("{initial_state}에서"));
            }
            if let Some(result_state) = result_state {
                phrases.push(with_directional_particle(&result_state));
            }
            match (date, time) {
                (Some(date), Some(time)) => phrases.push(format!("{date} {time}에")),
                (Some(date), None) => phrases.push(format!("{date}에")),
                (None, Some(time)) => phrases.push(format!("{time}에")),
                (None, None) => {}
            }
            if let Some(location) = location {
                phrases.push(format!("{location}에서"));
            }
            if let Some(instrument) = instrument {
                phrases.push(format!("{} 사용해", with_particle(&instrument, "을", "를")));
            }
            if let Some(manner) = manner {
                phrases.push(format!("{manner} 방식으로"));
            }
            let formal = response.style.register == LanguageRegisterIR::Formal;
            let ending = korean_semantic_event_predicate(
                predicate_sense,
                predicate.phase,
                information.voice,
                formal,
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
    let last = value
        .chars()
        .rev()
        .find(|character| !character.is_whitespace());
    let use_ro = last.is_some_and(|character| {
        let code = character as u32;
        !(0xAC00..=0xD7A3).contains(&code)
            || (code - 0xAC00).is_multiple_of(28)
            || (code - 0xAC00) % 28 == 8
    });
    format!("{value}{}", if use_ro { "로" } else { "으로" })
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
    english: EnglishEventLexeme,
}

const EVENT_LANGUAGE_CODECS: &[EventLanguageCodec] = &[
    EventLanguageCodec {
        predicate_sense: ApprovedEventPredicateSenseIR::Move,
        korean_nominal: "이동",
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
        predicate_sense: ApprovedEventPredicateSenseIR::Give,
        korean_nominal: "제공",
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
        predicate_sense: ApprovedEventPredicateSenseIR::Create,
        korean_nominal: "생성",
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
        predicate_sense: ApprovedEventPredicateSenseIR::Change,
        korean_nominal: "변경",
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
        predicate_sense: ApprovedEventPredicateSenseIR::Start,
        korean_nominal: "시작",
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
        predicate_sense: ApprovedEventPredicateSenseIR::Complete,
        korean_nominal: "완료",
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
        english: EnglishEventLexeme {
            base: "close",
            present: "closes",
            progressive: "closing",
            past: "closed",
            participle: "closed",
            noun: "closure",
        },
    },
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
    formal: bool,
) -> String {
    let nominal = event_language_codec(predicate_sense).korean_nominal;
    let suffix = match (voice, phase, formal) {
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Planned, true) => "할 예정입니다",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Planned, false) => "할 예정이에요",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Scheduled, true) => "합니다",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Scheduled, false) => "해요",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Ongoing, true) => " 중입니다",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Ongoing, false) => " 중이에요",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Completed, true) => "했습니다",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Completed, false) => "했어요",
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Cancelled, true) => {
            "할 예정이었지만 취소됐습니다"
        }
        (ApprovedEventVoiceIR::Active, ApprovedEventPhaseIR::Cancelled, false) => {
            "할 예정이었지만 취소됐어요"
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Planned, true) => "될 예정입니다",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Planned, false) => "될 예정이에요",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Scheduled, true) => "됩니다",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Scheduled, false) => "돼요",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Ongoing, true) => "되고 있습니다",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Ongoing, false) => "되고 있어요",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Completed, true) => "됐습니다",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Completed, false) => "됐어요",
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Cancelled, true) => {
            "될 예정이었지만 취소됐습니다"
        }
        (ApprovedEventVoiceIR::Passive, ApprovedEventPhaseIR::Cancelled, false) => {
            "될 예정이었지만 취소됐어요"
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
    formal: bool,
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
    Some(if formal { pair.0 } else { pair.1 })
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
    const ENDINGS: [(&str, &str); 13] = [
        ("되지 않았습니다", "되지 않았고,"),
        ("되지 않았어요", "되지 않았고,"),
        ("할 수 없습니다", "할 수 없고,"),
        ("할 수 없어요", "할 수 없고,"),
        ("할 수 있습니다", "할 수 있고,"),
        ("할 수 있어요", "할 수 있고,"),
        ("변경됐습니다", "변경됐고,"),
        ("변경됐어요", "변경됐고,"),
        ("아닙니다", "아니고,"),
        ("아니에요", "아니고,"),
        ("됐습니다", "됐고,"),
        ("됐어요", "됐고,"),
        ("입니다", "이고,"),
    ];
    for (finite, coordinated) in ENDINGS {
        if let Some(stem) = sentence.strip_suffix(finite) {
            return Some(format!("{stem}{coordinated}"));
        }
    }
    for (finite, coordinated) in [
        ("이에요", "이고,"),
        ("예요", "이고,"),
        ("맞습니다", "맞고,"),
        ("맞아요", "맞고,"),
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
        ) => Some("정정하면, "),
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
    let sentence = full.strip_suffix('.').unwrap_or(&full);
    match language {
        LanguageCodeIR::Korean => {
            let topic = object_particle(&claim.subject.canonical_lexical_label, "은", "는").0;
            let topic_prefix = format!("{}{topic} ", claim.subject.canonical_lexical_label);
            let plain_prefix = format!("{} ", claim.subject.canonical_lexical_label);
            sentence
                .strip_prefix(&topic_prefix)
                .or_else(|| sentence.strip_prefix(&plain_prefix))
                .map(|followup| format!("또한, {followup}."))
                .unwrap_or(full)
        }
        LanguageCodeIR::English => {
            let subject_prefix = format!("{} ", claim.subject.canonical_lexical_label);
            if let Some(predicate) = sentence.strip_prefix(&subject_prefix) {
                return format!("Also, it {predicate}.");
            }
            let relation = relation_label(claim.relation, LanguageCodeIR::English);
            let relational_prefix = format!(
                "The {relation} of {} ",
                claim.subject.canonical_lexical_label
            );
            sentence
                .strip_prefix(&relational_prefix)
                .map(|predicate| format!("Also, its {relation} {predicate}."))
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
    match language {
        LanguageCodeIR::Korean => realize_korean_claim(claim, response),
        _ => realize_english_claim(claim, response),
    }
}

fn realize_korean_claim(
    claim: &ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> String {
    let subject = claim.subject.canonical_lexical_label.trim();
    let relation = relation_label(claim.relation, LanguageCodeIR::Korean);
    let value = display_value(
        &claim.value,
        LanguageCodeIR::Korean,
        response.style.register,
    );
    let topic = object_particle(relation, "은", "는").0;
    let formal = response.style.register == LanguageRegisterIR::Formal;
    if let ApprovedOpenValueIR::Boolean(value) = &claim.value {
        let asserted = if !claim.polarity || response.operation == ApprovedOperationIR::Negate {
            !*value
        } else {
            *value
        };
        return realize_korean_boolean_claim(subject, claim.relation, asserted, formal);
    }
    if !claim.polarity || response.operation == ApprovedOperationIR::Negate {
        let ending = if formal {
            "아닙니다"
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
        } else {
            "변경됐어요"
        };
        return format!(
            "{subject} {relation}{topic}{} {ending}.",
            object_particle(particle_basis(&claim.value, &value), "으로", "로").prepend(&value)
        );
    }
    let ending = if formal {
        "입니다"
    } else {
        copula_yo(particle_basis(&claim.value, &value))
    };
    format!("{subject} {relation}{topic} {value}{ending}.")
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
    formal: bool,
) -> String {
    let subject_topic = object_particle(subject, "은", "는").0;
    let predicate = match (relation, asserted, formal) {
        (ApprovedRelationTypeIR::Cancelled, true, true) => "취소됐습니다",
        (ApprovedRelationTypeIR::Cancelled, true, false) => "취소됐어요",
        (ApprovedRelationTypeIR::Cancelled, false, true) => "취소되지 않았습니다",
        (ApprovedRelationTypeIR::Cancelled, false, false) => "취소되지 않았어요",
        (ApprovedRelationTypeIR::RoomAvailable, true, true) => "사용할 수 있습니다",
        (ApprovedRelationTypeIR::RoomAvailable, true, false) => "사용할 수 있어요",
        (ApprovedRelationTypeIR::RoomAvailable, false, true) => "사용할 수 없습니다",
        (ApprovedRelationTypeIR::RoomAvailable, false, false) => "사용할 수 없어요",
        (ApprovedRelationTypeIR::Registration, true, true) => "등록됐습니다",
        (ApprovedRelationTypeIR::Registration, true, false) => "등록됐어요",
        (ApprovedRelationTypeIR::Registration, false, true) => "등록되지 않았습니다",
        (ApprovedRelationTypeIR::Registration, false, false) => "등록되지 않았어요",
        (ApprovedRelationTypeIR::Entry, true, true) => "입장할 수 있습니다",
        (ApprovedRelationTypeIR::Entry, true, false) => "입장할 수 있어요",
        (ApprovedRelationTypeIR::Entry, false, true) => "입장할 수 없습니다",
        (ApprovedRelationTypeIR::Entry, false, false) => "입장할 수 없어요",
        (ApprovedRelationTypeIR::Confirmed, true, true) => "확정됐습니다",
        (ApprovedRelationTypeIR::Confirmed, true, false) => "확정됐어요",
        (ApprovedRelationTypeIR::Confirmed, false, true) => "확정되지 않았습니다",
        (ApprovedRelationTypeIR::Confirmed, false, false) => "확정되지 않았어요",
        _ => {
            let relation = relation_label(relation, LanguageCodeIR::Korean);
            let relation_topic = object_particle(relation, "은", "는").0;
            let truth = match (asserted, formal) {
                (true, true) => "맞습니다",
                (true, false) => "맞아요",
                (false, true) => "아닙니다",
                (false, false) => "아니에요",
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
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Capacity) => "정원",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Duration) => "소요 시간",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Name) => "이름",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Count) => "개수",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Date) => "날짜",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Quantity) => "수량",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Agent) => "행위자",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Patient) => "수령자",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Theme) => "대상물",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Source) => "출발점",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Destination) => "도착점",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Instrument) => "도구",
        (LanguageCodeIR::Korean, ApprovedRelationTypeIR::Manner) => "방식",
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
        (_, ApprovedRelationTypeIR::Capacity) => "capacity",
        (_, ApprovedRelationTypeIR::Duration) => "duration",
        (_, ApprovedRelationTypeIR::Name) => "name",
        (_, ApprovedRelationTypeIR::Count) => "count",
        (_, ApprovedRelationTypeIR::Date) => "date",
        (_, ApprovedRelationTypeIR::Quantity) => "quantity",
        (_, ApprovedRelationTypeIR::Agent) => "agent",
        (_, ApprovedRelationTypeIR::Patient) => "patient",
        (_, ApprovedRelationTypeIR::Theme) => "theme",
        (_, ApprovedRelationTypeIR::Source) => "source",
        (_, ApprovedRelationTypeIR::Destination) => "destination",
        (_, ApprovedRelationTypeIR::Instrument) => "instrument",
        (_, ApprovedRelationTypeIR::Manner) => "manner",
        (_, ApprovedRelationTypeIR::InitialState) => "initial state",
        (_, ApprovedRelationTypeIR::ResultState) => "result state",
    }
}

fn display_value(
    value: &ApprovedOpenValueIR,
    language: LanguageCodeIR,
    _register: LanguageRegisterIR,
) -> String {
    match value {
        ApprovedOpenValueIR::Lexical(node) => node.canonical_lexical_label.clone(),
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

fn has_final_consonant(value: &str) -> bool {
    let Some(character) = value
        .chars()
        .rev()
        .find(|character| !character.is_whitespace())
    else {
        return false;
    };
    let code = character as u32;
    (0xAC00..=0xD7A3).contains(&code) && !(code - 0xAC00).is_multiple_of(28)
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
        compositional_response_sha256, ApprovedClauseUnitIR, ApprovedEventDiscourseStateIR,
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
            .contains("회의 상태는 “준비 중”입니다. 따라서, 취소됐습니다."));

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
            "정정하면, 회의 시간은 오후 4시로 변경됐고, 장소는 회의실 B로 변경됐습니다."
        ));
        assert_eq!(korean.markdown.matches("정정하면,").count(), 1);

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
            "정정하면, 회의 시간은 오후 4시입니다.",
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
            "행사 상태는 “준비 중이고, 확인됨”이고, 취소되지 않았고, 등록됐습니다. 또한, 확정됐습니다."
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
            "민수가 지수에게 보고서를 보관함에서 서버로 2026년 10월 12일 오후 4시 30분에 작업실에서 USB를 사용해 “암호화” 방식으로 전송했습니다."
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
                "민수가 상자를 창고에서 작업실로 이동했습니다.",
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
    fn semantic_event_registry_contract_is_closed_order_independent_and_non_authoritative() {
        let specs = crate::approved_response::APPROVED_EVENT_PREDICATE_SPECS;
        assert_eq!(specs.len(), 11);
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
                DocumentResponseRoleIR::Identity,
                DocumentResponseRoleIR::Status,
                DocumentResponseRoleIR::Timeline,
                DocumentResponseRoleIR::PlaceAndAccess,
                DocumentResponseRoleIR::Measurement,
            ]
        );
        assert!(output.markdown.contains("## 이유와 설명"));
        for heading in [
            "### 대상 정보",
            "### 상태와 결정",
            "### 시간 정보",
            "### 장소와 이용",
            "### 수량과 규모",
        ] {
            assert!(output.markdown.contains(heading), "{heading}");
        }
        for continuation in [
            "행사는 2026년 10월 12일 오후 4시 30분에 B홀에서 열립니다.",
            "행사는 입장할 수 있습니다.",
            "정리하면, 개수는 8입니다.",
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
            "### Subject information",
            "### Status and decisions",
            "### Time information",
            "### Location and access",
            "### Quantities and scale",
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
        assert_eq!(output.plan.sections.len(), MAX_SECTIONS);
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
    fn bounded_long_document_roundtrips_sixty_four_claims() {
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
        assert!(output
            .markdown
            .contains("64. 정리하면, 항목 63 개수는 63입니다."));
        eprintln!(
            "B_CORE_DOCUMENT_64_CLAIM_ROUNDTRIP={}",
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
            "P_64",
            "item_64",
            "항목 64",
            ApprovedRelationTypeIR::Count,
            ApprovedOpenValueIR::Integer(64),
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
}
