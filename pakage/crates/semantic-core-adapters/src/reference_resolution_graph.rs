//! Bounded, inspectable graph for compositional reference resolution.
//!
//! Mention detection, candidate competition, and the selected bindings remain
//! adapter-local.  The graph is evidence about how language was grounded; it
//! is never semantic authority and never authorizes execution.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::conversation::DiscourseBindingIR;

pub const REFERENCE_RESOLUTION_GRAPH_SCHEMA: &str = "B_CORE_REFERENCE_RESOLUTION_GRAPH_IR_1";
pub const MAX_REFERENCE_MENTIONS: usize = 32;
pub const MAX_REFERENCE_CANDIDATES: usize = 32;
pub const MAX_REFERENCE_EDGES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReferenceMentionKindIR {
    Possessive,
    Demonstrative,
    Ordered,
    PersonPronoun,
    GenericPronoun,
    ZeroArgumentEllipsis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceMentionNodeIR {
    pub mention_id: String,
    pub source_surface: String,
    pub normalized_surface: String,
    pub kind: ReferenceMentionKindIR,
    pub byte_start: usize,
    pub byte_end: usize,
    pub clause_index: usize,
    pub required_semantic_type: String,
    pub quote_inert: bool,
    pub implicit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceAntecedentCandidateIR {
    pub antecedent_id: String,
    pub antecedent_surface: String,
    pub semantic_type: String,
    pub source: String,
    pub salience_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceCandidateEdgeIR {
    pub edge_id: String,
    pub mention_id: String,
    pub antecedent_id: String,
    pub antecedent_surface: String,
    pub candidate_source: String,
    pub type_compatible: bool,
    pub score_millis: u16,
    pub selected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceSelectionHint {
    pub mention_byte_start: usize,
    pub antecedent_id: String,
    pub antecedent_surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceResolutionGraphIR {
    pub schema: String,
    pub source_text_sha256: String,
    pub resolution_sha256: String,
    pub mention_nodes: Vec<ReferenceMentionNodeIR>,
    pub antecedent_candidates: Vec<ReferenceAntecedentCandidateIR>,
    pub candidate_edges: Vec<ReferenceCandidateEdgeIR>,
    pub selected_binding_ids: Vec<String>,
    pub unresolved_mention_ids: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
    pub graph_sha256: String,
}

impl Default for ReferenceResolutionGraphIR {
    fn default() -> Self {
        build_reference_resolution_graph("", "", &[], &[], &[])
    }
}

impl ReferenceResolutionGraphIR {
    pub fn validate(&self) -> bool {
        if self.schema != REFERENCE_RESOLUTION_GRAPH_SCHEMA
            || self.source_text_sha256.len() != 64
            || self.resolution_sha256.len() != 64
            || self.graph_sha256.len() != 64
            || self.mention_nodes.len() > MAX_REFERENCE_MENTIONS
            || self.antecedent_candidates.len() > MAX_REFERENCE_CANDIDATES
            || self.candidate_edges.len() > MAX_REFERENCE_EDGES
            || self.semantic_authority
            || self.external_execution_authorized
        {
            return false;
        }
        let mention_ids = self
            .mention_nodes
            .iter()
            .map(|mention| mention.mention_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let candidate_ids = self
            .antecedent_candidates
            .iter()
            .map(|candidate| candidate.antecedent_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if mention_ids.len() != self.mention_nodes.len()
            || candidate_ids.len() != self.antecedent_candidates.len()
            || self.candidate_edges.iter().any(|edge| {
                !mention_ids.contains(edge.mention_id.as_str())
                    || !candidate_ids.contains(edge.antecedent_id.as_str())
            })
            || self
                .unresolved_mention_ids
                .iter()
                .any(|mention_id| !mention_ids.contains(mention_id.as_str()))
        {
            return false;
        }
        let mut canonical = self.clone();
        canonical.graph_sha256.clear();
        self.graph_sha256 == hash_json(&canonical)
    }

    pub fn validate_against(
        &self,
        source_text: &str,
        resolved_text: &str,
        binding_count: usize,
    ) -> bool {
        let selected_edges = self
            .candidate_edges
            .iter()
            .filter(|edge| edge.selected)
            .collect::<Vec<_>>();
        let selected_edge_ids = selected_edges
            .iter()
            .map(|edge| edge.edge_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let recorded_ids = self
            .selected_binding_ids
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let resolved_mentions = selected_edges
            .iter()
            .map(|edge| edge.mention_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        self.validate()
            && self.source_text_sha256 == hash_bytes(source_text.as_bytes())
            && self.resolution_sha256 == hash_bytes(resolved_text.as_bytes())
            && selected_edge_ids == recorded_ids
            && selected_edge_ids.len() <= binding_count
            && self
                .unresolved_mention_ids
                .iter()
                .all(|mention_id| !resolved_mentions.contains(mention_id.as_str()))
    }
}

pub fn scan_reference_mentions(text: &str) -> Vec<ReferenceMentionNodeIR> {
    let markers = [
        (
            "that object",
            ReferenceMentionKindIR::Demonstrative,
            "ENTITY",
        ),
        ("that item", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("that one", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("그 대상을", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("그 항목을", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("그 객체를", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("그 대상이", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("그 항목이", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("그 객체가", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("그 대상", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("그 항목", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("그 객체", ReferenceMentionKindIR::Demonstrative, "ENTITY"),
        ("그것의", ReferenceMentionKindIR::Possessive, "ENTITY"),
        ("그거의", ReferenceMentionKindIR::Possessive, "ENTITY"),
        ("전자를", ReferenceMentionKindIR::Ordered, "ENTITY"),
        ("후자를", ReferenceMentionKindIR::Ordered, "ENTITY"),
        ("former", ReferenceMentionKindIR::Ordered, "ENTITY"),
        ("latter", ReferenceMentionKindIR::Ordered, "ENTITY"),
        ("전자", ReferenceMentionKindIR::Ordered, "ENTITY"),
        ("후자", ReferenceMentionKindIR::Ordered, "ENTITY"),
        ("its", ReferenceMentionKindIR::Possessive, "ENTITY"),
        ("her", ReferenceMentionKindIR::PersonPronoun, "PERSON"),
        ("his", ReferenceMentionKindIR::PersonPronoun, "PERSON"),
    ];
    let lower = text.to_lowercase();
    let mut raw = Vec::new();
    for (marker, kind, required_type) in markers {
        for (start, _) in lower.match_indices(marker) {
            let end = start + marker.len();
            if !text.is_char_boundary(start)
                || !text.is_char_boundary(end)
                || !boundary_before(&lower, start)
                || !boundary_after(&lower, end)
            {
                continue;
            }
            raw.push((
                start,
                end,
                marker.len(),
                kind,
                required_type,
                quote_state_at(text, start),
            ));
        }
    }
    raw.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.3.cmp(&right.3))
    });
    let mut accepted: Vec<(usize, usize, usize, ReferenceMentionKindIR, &str, bool)> = Vec::new();
    for candidate in raw {
        if accepted
            .iter()
            .any(|existing| candidate.0 < existing.1 && existing.0 < candidate.1)
        {
            continue;
        }
        accepted.push(candidate);
    }
    accepted.sort_by_key(|candidate| candidate.0);
    accepted.truncate(MAX_REFERENCE_MENTIONS);
    accepted
        .into_iter()
        .enumerate()
        .map(
            |(index, (start, end, _, kind, required_type, quote_inert))| ReferenceMentionNodeIR {
                mention_id: format!("REF-MENTION-{index:03}"),
                source_surface: text[start..end].to_string(),
                normalized_surface: lower[start..end].to_string(),
                kind,
                byte_start: start,
                byte_end: end,
                clause_index: clause_index(text, start),
                required_semantic_type: required_type.to_string(),
                quote_inert,
                implicit: false,
            },
        )
        .collect()
}

pub fn build_reference_resolution_graph(
    source_text: &str,
    resolved_text: &str,
    bindings: &[DiscourseBindingIR],
    candidates: &[ReferenceAntecedentCandidateIR],
    selection_hints: &[ReferenceSelectionHint],
) -> ReferenceResolutionGraphIR {
    let mentions = scan_reference_mentions(source_text);
    let mut bounded_candidates = candidates.to_vec();
    bounded_candidates.sort_by(|left, right| {
        right
            .salience_millis
            .cmp(&left.salience_millis)
            .then_with(|| left.antecedent_id.cmp(&right.antecedent_id))
    });
    bounded_candidates.dedup_by(|left, right| left.antecedent_id == right.antecedent_id);
    bounded_candidates.truncate(MAX_REFERENCE_CANDIDATES);
    let mut edges = Vec::new();
    let mut selected_ids = Vec::new();
    let mut unresolved = Vec::new();
    for mention in &mentions {
        if mention.quote_inert {
            continue;
        }
        let selected_hint = selection_hints
            .iter()
            .find(|hint| hint.mention_byte_start == mention.byte_start);
        let mut mention_selected = false;
        for candidate in &bounded_candidates {
            if edges.len() >= MAX_REFERENCE_EDGES {
                break;
            }
            let compatible = mention.required_semantic_type == "ENTITY"
                || candidate.semantic_type == mention.required_semantic_type;
            let selected = selected_hint
                .is_some_and(|hint| hint.antecedent_id == candidate.antecedent_id && compatible);
            let edge_id = format!("REF-EDGE-{:03}", edges.len());
            if selected {
                mention_selected = true;
                selected_ids.push(edge_id.clone());
            }
            edges.push(ReferenceCandidateEdgeIR {
                edge_id,
                mention_id: mention.mention_id.clone(),
                antecedent_id: candidate.antecedent_id.clone(),
                antecedent_surface: if selected {
                    selected_hint
                        .map(|hint| hint.antecedent_surface.clone())
                        .unwrap_or_else(|| candidate.antecedent_surface.clone())
                } else {
                    candidate.antecedent_surface.clone()
                },
                candidate_source: candidate.source.clone(),
                type_compatible: compatible,
                score_millis: if selected {
                    candidate.salience_millis
                } else {
                    candidate.salience_millis.saturating_sub(100)
                },
                selected,
                rejection_reason: (!selected).then(|| {
                    if compatible {
                        "LOWER_ROUTING_SCORE".to_string()
                    } else {
                        "TYPE_MISMATCH".to_string()
                    }
                }),
            });
        }
        if !mention_selected {
            unresolved.push(mention.mention_id.clone());
        }
    }
    if selection_hints.is_empty() && bindings.is_empty() {
        selected_ids.clear();
    }
    let mut graph = ReferenceResolutionGraphIR {
        schema: REFERENCE_RESOLUTION_GRAPH_SCHEMA.to_string(),
        source_text_sha256: hash_bytes(source_text.as_bytes()),
        resolution_sha256: hash_bytes(resolved_text.as_bytes()),
        mention_nodes: mentions,
        antecedent_candidates: bounded_candidates,
        candidate_edges: edges,
        selected_binding_ids: selected_ids,
        unresolved_mention_ids: unresolved,
        semantic_authority: false,
        external_execution_authorized: false,
        graph_sha256: String::new(),
    };
    graph.graph_sha256 = hash_json(&graph);
    graph
}

fn hash_json<T: Serialize>(value: &T) -> String {
    hash_bytes(&serde_json::to_vec(value).expect("reference graph serialization"))
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn boundary_before(text: &str, index: usize) -> bool {
    index == 0
        || text[..index]
            .chars()
            .next_back()
            .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn boundary_after(text: &str, index: usize) -> bool {
    index == text.len()
        || text[index..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn quote_state_at(text: &str, index: usize) -> bool {
    let mut curly_single = false;
    let mut curly_double = false;
    let mut ascii_double = false;
    for character in text[..index].chars() {
        match character {
            '‘' => curly_single = true,
            '’' => curly_single = false,
            '“' => curly_double = true,
            '”' => curly_double = false,
            '"' => ascii_double = !ascii_double,
            _ => {}
        }
    }
    curly_single || curly_double || ascii_double
}

fn clause_index(text: &str, end: usize) -> usize {
    text[..end]
        .chars()
        .filter(|character| matches!(character, '.' | ';' | '!' | '?'))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_repeated_and_post_quote_mentions_without_overlap() {
        let mentions = scan_reference_mentions(
            "The label says ‘its state’; compare its owner with that object's status.",
        );
        assert_eq!(mentions.len(), 3);
        assert!(mentions[0].quote_inert);
        assert!(!mentions[1].quote_inert);
        assert_eq!(mentions[2].source_surface.to_lowercase(), "that object");
    }

    #[test]
    fn default_graph_is_hash_valid_and_non_authoritative() {
        let graph = ReferenceResolutionGraphIR::default();
        assert!(graph.validate());
        assert!(!graph.semantic_authority);
        assert!(!graph.external_execution_authorized);
    }

    #[test]
    fn validation_rejects_bound_text_and_selected_edge_tampering() {
        let source = "compare its state";
        let resolved = "compare cache state";
        let candidates = vec![ReferenceAntecedentCandidateIR {
            antecedent_id: "REF-cache".to_string(),
            antecedent_surface: "cache".to_string(),
            semantic_type: "ENTITY".to_string(),
            source: "ACTIVE_FOCUS".to_string(),
            salience_millis: 900,
        }];
        let hints = vec![ReferenceSelectionHint {
            mention_byte_start: source.find("its").expect("possessive marker"),
            antecedent_id: "REF-cache".to_string(),
            antecedent_surface: "cache".to_string(),
        }];
        let graph = build_reference_resolution_graph(source, resolved, &[], &candidates, &hints);
        assert!(graph.validate_against(source, resolved, 1));
        assert!(!graph.validate_against("compare its owner", resolved, 1));
        assert!(!graph.validate_against(source, "compare queue state", 1));

        let mut tampered = graph.clone();
        tampered.selected_binding_ids.clear();
        tampered.graph_sha256.clear();
        tampered.graph_sha256 = hash_json(&tampered);
        assert!(tampered.validate());
        assert!(!tampered.validate_against(source, resolved, 1));
    }
}
