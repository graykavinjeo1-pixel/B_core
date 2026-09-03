//! Typed clause boundaries and relations for deterministic language composition.
//!
//! This graph is discourse-local syntax/semantics evidence. Connector surfaces
//! select typed relations, but never become semantic concepts or execution
//! authority by themselves.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::compositional_semantics::PredicateFrameIR;

pub const CLAUSE_GRAPH_SCHEMA: &str = "B_CORE_CLAUSE_GRAPH_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClauseFunctionIR {
    Main,
    Coordinate,
    Condition,
    Cause,
    Purpose,
    Concession,
    Temporal,
}

impl ClauseFunctionIR {
    pub fn permits_independent_directive(self) -> bool {
        matches!(self, Self::Main | Self::Coordinate)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClauseRelationKindIR {
    Coordination,
    Sequence,
    Condition,
    Cause,
    Purpose,
    Contrast,
    TemporalBefore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClauseNodeIR {
    pub clause_id: String,
    pub anchor_frame_id: String,
    pub canonical_predicate: String,
    pub function: ClauseFunctionIR,
    pub source_start_byte: usize,
    pub source_end_byte: usize,
    pub source_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClauseRelationEdgeIR {
    pub source_clause_id: String,
    pub target_clause_id: String,
    pub relation: ClauseRelationKindIR,
    pub marker_surface: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClauseGraphIR {
    pub schema: String,
    pub nodes: Vec<ClauseNodeIR>,
    pub edges: Vec<ClauseRelationEdgeIR>,
    pub unresolved_connectors: Vec<String>,
}

impl Default for ClauseGraphIR {
    fn default() -> Self {
        Self {
            schema: CLAUSE_GRAPH_SCHEMA.to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            unresolved_connectors: Vec::new(),
        }
    }
}

impl ClauseGraphIR {
    pub fn node_for_frame(&self, frame_id: &str) -> Option<&ClauseNodeIR> {
        self.nodes
            .iter()
            .find(|node| node.anchor_frame_id == frame_id)
    }

    pub fn relation_between_frames(
        &self,
        source_frame_id: &str,
        target_frame_id: &str,
    ) -> Option<ClauseRelationKindIR> {
        let source = self.node_for_frame(source_frame_id)?;
        let target = self.node_for_frame(target_frame_id)?;
        self.edges
            .iter()
            .find(|edge| {
                edge.source_clause_id == source.clause_id
                    && edge.target_clause_id == target.clause_id
            })
            .map(|edge| edge.relation)
    }

    pub fn validate(&self, text: &str) -> bool {
        if self.schema != CLAUSE_GRAPH_SCHEMA {
            return false;
        }
        let clause_ids = self
            .nodes
            .iter()
            .map(|node| &node.clause_id)
            .collect::<BTreeSet<_>>();
        let frame_ids = self
            .nodes
            .iter()
            .map(|node| &node.anchor_frame_id)
            .collect::<BTreeSet<_>>();
        if clause_ids.len() != self.nodes.len()
            || frame_ids.len() != self.nodes.len()
            || self.nodes.iter().any(|node| {
                node.clause_id.trim().is_empty()
                    || node.anchor_frame_id.trim().is_empty()
                    || node.canonical_predicate.trim().is_empty()
                    || node.source_text.trim().is_empty()
                    || node.source_start_byte >= node.source_end_byte
                    || node.source_end_byte > text.len()
                    || !text.is_char_boundary(node.source_start_byte)
                    || !text.is_char_boundary(node.source_end_byte)
            })
        {
            return false;
        }
        self.edges.iter().all(|edge| {
            clause_ids.contains(&edge.source_clause_id)
                && clause_ids.contains(&edge.target_clause_id)
                && edge.source_clause_id != edge.target_clause_id
                && !edge.marker_surface.trim().is_empty()
                && edge.confidence_millis <= 1_000
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ClauseStructureAnalyzer;

impl ClauseStructureAnalyzer {
    pub fn analyze(&self, text: &str, frames: &[PredicateFrameIR]) -> ClauseGraphIR {
        if frames.is_empty() {
            return ClauseGraphIR::default();
        }
        let mut ordered = frames.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|frame| frame.source_start_byte);
        let mut nodes = ordered
            .iter()
            .enumerate()
            .map(|(index, frame)| {
                let (start, end) = sentence_bounds(text, frame.source_start_byte);
                ClauseNodeIR {
                    clause_id: format!("GRAMMAR-CLAUSE-{:02}", index + 1),
                    anchor_frame_id: frame.frame_id.clone(),
                    canonical_predicate: frame.canonical_predicate.clone(),
                    function: ClauseFunctionIR::Main,
                    source_start_byte: start,
                    source_end_byte: end,
                    source_text: String::new(),
                }
            })
            .collect::<Vec<_>>();
        let mut edges = Vec::new();
        let mut unresolved = Vec::new();

        for index in 0..ordered.len().saturating_sub(1) {
            let left = ordered[index];
            let right = ordered[index + 1];
            let (sentence_start, sentence_end) = sentence_bounds(text, left.source_start_byte);
            if right.source_start_byte >= sentence_end {
                continue;
            }
            let left_end = left
                .source_start_byte
                .saturating_add(left.predicate_surface.len());
            if left_end > right.source_start_byte || right.source_start_byte > text.len() {
                continue;
            }
            let prefix = safe_slice(text, sentence_start, left.source_start_byte);
            let between = safe_slice(text, left_end, right.source_start_byte);
            let Some(detection) = detect_relation(prefix, between, sentence_start, left_end) else {
                let residue = between.trim();
                if !residue.is_empty()
                    && left.mood != crate::compositional_semantics::FrameMoodIR::RelativeClause
                    && right.mood != crate::compositional_semantics::FrameMoodIR::RelativeClause
                {
                    unresolved.push(residue.to_string());
                }
                continue;
            };
            apply_function(&mut nodes[index].function, detection.left_function);
            apply_function(&mut nodes[index + 1].function, detection.right_function);
            if detection.prefix_marker {
                nodes[index].source_start_byte = nodes[index]
                    .source_start_byte
                    .max(detection.marker_end_byte);
                if let Some(comma) = between.find(',') {
                    let comma_byte = left_end + comma;
                    nodes[index].source_end_byte = nodes[index].source_end_byte.min(comma_byte);
                    nodes[index + 1].source_start_byte =
                        nodes[index + 1].source_start_byte.max(comma_byte + 1);
                }
            } else {
                nodes[index].source_end_byte = nodes[index]
                    .source_end_byte
                    .min(detection.marker_start_byte);
                nodes[index + 1].source_start_byte = nodes[index + 1]
                    .source_start_byte
                    .max(detection.marker_end_byte);
            }
            let (source, target) = if detection.source_is_left {
                (index, index + 1)
            } else {
                (index + 1, index)
            };
            edges.push(ClauseRelationEdgeIR {
                source_clause_id: nodes[source].clause_id.clone(),
                target_clause_id: nodes[target].clause_id.clone(),
                relation: detection.relation,
                marker_surface: detection.marker_surface,
                confidence_millis: detection.confidence_millis,
            });
        }

        for node in &mut nodes {
            trim_node_span(text, node);
        }
        edges.sort_by(|left, right| {
            left.source_clause_id
                .cmp(&right.source_clause_id)
                .then_with(|| left.target_clause_id.cmp(&right.target_clause_id))
                .then_with(|| left.relation.cmp(&right.relation))
        });
        edges.dedup_by(|left, right| {
            left.source_clause_id == right.source_clause_id
                && left.target_clause_id == right.target_clause_id
                && left.relation == right.relation
        });
        unresolved.sort();
        unresolved.dedup();
        let graph = ClauseGraphIR {
            schema: CLAUSE_GRAPH_SCHEMA.to_string(),
            nodes,
            edges,
            unresolved_connectors: unresolved,
        };
        debug_assert!(graph.validate(text));
        graph
    }
}

#[derive(Debug)]
struct RelationDetection {
    relation: ClauseRelationKindIR,
    source_is_left: bool,
    left_function: ClauseFunctionIR,
    right_function: ClauseFunctionIR,
    marker_start_byte: usize,
    marker_end_byte: usize,
    marker_surface: String,
    prefix_marker: bool,
    confidence_millis: u16,
}

fn detect_relation(
    prefix: &str,
    between: &str,
    prefix_start: usize,
    between_start: usize,
) -> Option<RelationDetection> {
    let prefix_lower = prefix.to_lowercase();
    let trimmed_start = prefix_lower
        .len()
        .saturating_sub(prefix_lower.trim_start().len());
    let trimmed = &prefix_lower[trimmed_start..];
    for (markers, relation, source_is_left, left_function) in [
        (
            &["provided that ", "unless ", "if "][..],
            ClauseRelationKindIR::Condition,
            true,
            ClauseFunctionIR::Condition,
        ),
        (
            &["because ", "since "][..],
            ClauseRelationKindIR::Cause,
            true,
            ClauseFunctionIR::Cause,
        ),
        (
            &["even though ", "although ", "though "][..],
            ClauseRelationKindIR::Contrast,
            true,
            ClauseFunctionIR::Concession,
        ),
        (
            &["before "][..],
            ClauseRelationKindIR::TemporalBefore,
            false,
            ClauseFunctionIR::Temporal,
        ),
        (
            &["after "][..],
            ClauseRelationKindIR::TemporalBefore,
            true,
            ClauseFunctionIR::Temporal,
        ),
        (
            &["when ", "once "][..],
            ClauseRelationKindIR::Condition,
            true,
            ClauseFunctionIR::Condition,
        ),
    ] {
        if let Some(marker) = markers.iter().find(|marker| trimmed.starts_with(**marker)) {
            let marker_start = prefix_start + trimmed_start;
            return Some(RelationDetection {
                relation,
                source_is_left,
                left_function,
                right_function: ClauseFunctionIR::Main,
                marker_start_byte: marker_start,
                marker_end_byte: marker_start + marker.len(),
                marker_surface: marker.trim().to_string(),
                prefix_marker: true,
                confidence_millis: 950,
            });
        }
    }

    let between_lower = between.to_lowercase();
    let between_trimmed_start = between_lower
        .len()
        .saturating_sub(between_lower.trim_start().len());
    let between_trimmed = &between_lower[between_trimmed_start..];
    let coordinated_condition = if let Some(local_start) = between_lower.find("and if ") {
        Some((local_start, "and"))
    } else if between_trimmed.starts_with("하고 ") {
        Some((between_trimmed_start, "하고"))
    } else if between_trimmed.starts_with("고 ") {
        Some((between_trimmed_start, "고"))
    } else {
        None
    };
    let contains_dependent_condition = between_trimmed.contains(" if ")
        || between_trimmed.starts_with("and if ")
        || ["경우에만", "경우", "한다면", "하면", "다면", "면"]
            .iter()
            .any(|marker| between_trimmed.contains(marker));
    if let Some((local_start, marker)) =
        coordinated_condition.filter(|_| contains_dependent_condition)
    {
        let marker_start = between_start + local_start;
        return Some(RelationDetection {
            relation: ClauseRelationKindIR::Sequence,
            source_is_left: true,
            left_function: ClauseFunctionIR::Main,
            right_function: ClauseFunctionIR::Coordinate,
            marker_start_byte: marker_start,
            marker_end_byte: marker_start + marker.len(),
            marker_surface: marker.to_string(),
            prefix_marker: false,
            confidence_millis: 930,
        });
    }
    let specifications = [
        MarkerSpec::new(
            &["하기 위해", "기 위해", "하도록"],
            ClauseRelationKindIR::Purpose,
            false,
            ClauseFunctionIR::Purpose,
            ClauseFunctionIR::Main,
            940,
        ),
        MarkerSpec::new(
            &[" in order to ", " so that "],
            ClauseRelationKindIR::Purpose,
            true,
            ClauseFunctionIR::Main,
            ClauseFunctionIR::Purpose,
            940,
        ),
        MarkerSpec::new(
            &[
                "기 때문에",
                "때문에",
                "했기 때문에",
                "해서",
                "하므로",
                "므로",
            ],
            ClauseRelationKindIR::Cause,
            true,
            ClauseFunctionIR::Cause,
            ClauseFunctionIR::Main,
            930,
        ),
        MarkerSpec::new(
            &[" because ", " since "],
            ClauseRelationKindIR::Cause,
            false,
            ClauseFunctionIR::Main,
            ClauseFunctionIR::Cause,
            930,
        ),
        MarkerSpec::new(
            &["경우에만", "경우", "한다면", "하면", "다면", "면"],
            ClauseRelationKindIR::Condition,
            true,
            ClauseFunctionIR::Condition,
            ClauseFunctionIR::Main,
            930,
        ),
        MarkerSpec::new(
            &[" provided that ", " unless ", " if "],
            ClauseRelationKindIR::Condition,
            false,
            ClauseFunctionIR::Main,
            ClauseFunctionIR::Condition,
            940,
        ),
        MarkerSpec::new(
            &["하기 전에", "기 전에", " 전에"],
            ClauseRelationKindIR::TemporalBefore,
            false,
            ClauseFunctionIR::Temporal,
            ClauseFunctionIR::Main,
            940,
        ),
        MarkerSpec::new(
            &[" before "],
            ClauseRelationKindIR::TemporalBefore,
            true,
            ClauseFunctionIR::Main,
            ClauseFunctionIR::Temporal,
            940,
        ),
        MarkerSpec::new(
            &[" after "],
            ClauseRelationKindIR::TemporalBefore,
            false,
            ClauseFunctionIR::Main,
            ClauseFunctionIR::Temporal,
            940,
        ),
        MarkerSpec::new(
            &["고 나서", "한 뒤", "한 후"],
            ClauseRelationKindIR::TemporalBefore,
            true,
            ClauseFunctionIR::Main,
            ClauseFunctionIR::Coordinate,
            950,
        ),
        MarkerSpec::new(
            &["했지만", "지만", "반면"],
            ClauseRelationKindIR::Contrast,
            true,
            ClauseFunctionIR::Concession,
            ClauseFunctionIR::Main,
            920,
        ),
        MarkerSpec::new(
            &[" but ", " however "],
            ClauseRelationKindIR::Contrast,
            true,
            ClauseFunctionIR::Main,
            ClauseFunctionIR::Coordinate,
            920,
        ),
        MarkerSpec::new(
            &[" then ", ", then "],
            ClauseRelationKindIR::Sequence,
            true,
            ClauseFunctionIR::Main,
            ClauseFunctionIR::Coordinate,
            920,
        ),
        MarkerSpec::new(
            &[" and "],
            ClauseRelationKindIR::Coordination,
            true,
            ClauseFunctionIR::Main,
            ClauseFunctionIR::Coordinate,
            900,
        ),
        MarkerSpec::new(
            &["하고", "고"],
            ClauseRelationKindIR::Sequence,
            true,
            ClauseFunctionIR::Main,
            ClauseFunctionIR::Coordinate,
            880,
        ),
    ];
    for specification in specifications {
        if let Some((local_start, marker)) = find_marker(&between_lower, specification.markers) {
            let marker_start = between_start + local_start;
            return Some(RelationDetection {
                relation: specification.relation,
                source_is_left: specification.source_is_left,
                left_function: specification.left_function,
                right_function: specification.right_function,
                marker_start_byte: marker_start,
                marker_end_byte: marker_start + marker.len(),
                marker_surface: marker.trim().to_string(),
                prefix_marker: false,
                confidence_millis: specification.confidence_millis,
            });
        }
    }
    let comma_only = between_trimmed.trim();
    if !comma_only.is_empty()
        && comma_only
            .chars()
            .all(|character| character == ',' || character.is_whitespace())
    {
        let local_start = between_lower.find(',').unwrap_or_default();
        let marker_start = between_start + local_start;
        return Some(RelationDetection {
            relation: ClauseRelationKindIR::Coordination,
            source_is_left: true,
            left_function: ClauseFunctionIR::Main,
            right_function: ClauseFunctionIR::Coordinate,
            marker_start_byte: marker_start,
            marker_end_byte: marker_start + 1,
            marker_surface: ",".to_string(),
            prefix_marker: false,
            confidence_millis: 840,
        });
    }
    None
}

#[derive(Debug, Clone, Copy)]
struct MarkerSpec {
    markers: &'static [&'static str],
    relation: ClauseRelationKindIR,
    source_is_left: bool,
    left_function: ClauseFunctionIR,
    right_function: ClauseFunctionIR,
    confidence_millis: u16,
}

impl MarkerSpec {
    const fn new(
        markers: &'static [&'static str],
        relation: ClauseRelationKindIR,
        source_is_left: bool,
        left_function: ClauseFunctionIR,
        right_function: ClauseFunctionIR,
        confidence_millis: u16,
    ) -> Self {
        Self {
            markers,
            relation,
            source_is_left,
            left_function,
            right_function,
            confidence_millis,
        }
    }
}

fn find_marker<'a>(text: &str, markers: &'a [&'a str]) -> Option<(usize, &'a str)> {
    markers
        .iter()
        .filter_map(|marker| text.find(marker).map(|position| (position, *marker)))
        .min_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| right.1.len().cmp(&left.1.len()))
        })
}

fn apply_function(current: &mut ClauseFunctionIR, proposed: ClauseFunctionIR) {
    if function_priority(proposed) > function_priority(*current) {
        *current = proposed;
    }
}

fn function_priority(function: ClauseFunctionIR) -> u8 {
    match function {
        ClauseFunctionIR::Main => 0,
        ClauseFunctionIR::Coordinate => 1,
        ClauseFunctionIR::Temporal => 2,
        ClauseFunctionIR::Cause => 3,
        ClauseFunctionIR::Purpose => 4,
        ClauseFunctionIR::Concession => 5,
        ClauseFunctionIR::Condition => 6,
    }
}

fn trim_node_span(text: &str, node: &mut ClauseNodeIR) {
    if node.source_start_byte >= node.source_end_byte || node.source_end_byte > text.len() {
        return;
    }
    let slice = &text[node.source_start_byte..node.source_end_byte];
    let leading = slice
        .len()
        .saturating_sub(slice.trim_start_matches(is_edge_noise).len());
    let trailing = slice
        .len()
        .saturating_sub(slice.trim_end_matches(is_edge_noise).len());
    node.source_start_byte += leading;
    node.source_end_byte = node.source_end_byte.saturating_sub(trailing);
    if node.source_start_byte < node.source_end_byte {
        node.source_text = text[node.source_start_byte..node.source_end_byte].to_string();
    }
}

fn is_edge_noise(character: char) -> bool {
    character.is_whitespace() || matches!(character, ',' | ':' | ';')
}

fn sentence_bounds(text: &str, position: usize) -> (usize, usize) {
    let start = text[..position]
        .char_indices()
        .rev()
        .find(|(_, character)| is_sentence_boundary(*character))
        .map_or(0, |(index, character)| index + character.len_utf8());
    let end = text[position..]
        .char_indices()
        .find(|(_, character)| is_sentence_boundary(*character))
        .map_or(text.len(), |(offset, _)| position + offset);
    (start, end)
}

fn is_sentence_boundary(character: char) -> bool {
    matches!(character, '.' | '?' | '!' | ';' | '\n' | '\r')
}

fn safe_slice(text: &str, start: usize, end: usize) -> &str {
    if start <= end
        && end <= text.len()
        && text.is_char_boundary(start)
        && text.is_char_boundary(end)
    {
        &text[start..end]
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositional_semantics::CompositionalSemanticAnalyzer;

    #[test]
    fn fronted_and_postposed_condition_have_the_same_direction() {
        let left =
            CompositionalSemanticAnalyzer.analyze("if you inspect the manifest, repair the parser");
        let right =
            CompositionalSemanticAnalyzer.analyze("repair the parser if you inspect the manifest");
        let signature = |graph: &ClauseGraphIR| {
            graph
                .edges
                .iter()
                .map(|edge| edge.relation)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            signature(&left.clause_graph),
            signature(&right.clause_graph)
        );
        assert_eq!(
            signature(&left.clause_graph),
            vec![ClauseRelationKindIR::Condition]
        );
    }

    #[test]
    fn comma_between_predicates_is_typed_coordination() {
        let analysis = CompositionalSemanticAnalyzer.analyze("Read, transform, and save the file.");
        assert_eq!(analysis.clause_graph.edges.len(), 2);
        assert!(analysis
            .clause_graph
            .edges
            .iter()
            .all(|edge| edge.relation == ClauseRelationKindIR::Coordination));
    }
}
