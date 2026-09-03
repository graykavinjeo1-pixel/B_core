//! Typed semantic-role and proposition graph for the language adapter.
//!
//! Nodes in this graph are discourse-local bindings, not promoted semantic
//! concepts. Unknown noun phrases remain surface-grounded entity nodes with no
//! invented concept ID. The graph records who did what to which object, role
//! particles/prepositions, quantifier scope, and relations between events.

use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};

use crate::clause_graph::{ClauseGraphIR, ClauseRelationKindIR};
use crate::compositional_semantics::{FrameMoodIR, PredicateFrameIR};

pub const SEMANTIC_ROLE_GRAPH_SCHEMA: &str = "B_CORE_SEMANTIC_ROLE_GRAPH_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticNodeKindIR {
    Event,
    Entity,
    ImplicitAgent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticNodeIR {
    pub node_id: String,
    pub kind: SemanticNodeKindIR,
    pub surface: String,
    pub normalized_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept_id_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_frame_id: Option<String>,
    pub source_clause_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticRoleKindIR {
    Agent,
    Topic,
    Theme,
    CoTheme,
    Patient,
    Experiencer,
    Recipient,
    Source,
    Destination,
    Instrument,
    Location,
    Result,
    ComparisonPeer,
    PriorResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticRoleEdgeIR {
    pub event_node_id: String,
    pub argument_node_id: String,
    pub role: SemanticRoleKindIR,
    pub evidence_surface: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SharedArgumentDirectionIR {
    Forward,
    Backward,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedArgumentBindingIR {
    pub binding_id: String,
    pub provider_event_node_id: String,
    pub dependent_event_node_id: String,
    pub argument_node_id: String,
    pub role: SemanticRoleKindIR,
    pub direction: SharedArgumentDirectionIR,
    pub relation: ClauseRelationKindIR,
    pub evidence_surface: String,
    pub confidence_millis: u16,
    pub syntactically_licensed: bool,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventRelationKindIR {
    Coordination,
    Cause,
    Condition,
    Purpose,
    TemporalBefore,
    Contrast,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRelationEdgeIR {
    pub source_event_node_id: String,
    pub target_event_node_id: String,
    pub relation: EventRelationKindIR,
    pub evidence_surface: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuantifierKindIR {
    All,
    Each,
    Some,
    Any,
    None,
    Exactly,
    AtLeast,
    AtMost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantifierScopeIR {
    pub target_node_id: String,
    pub quantifier: QuantifierKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardinality: Option<u64>,
    pub negated: bool,
    pub evidence_surface: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelativeClauseAttachmentIR {
    pub attachment_id: String,
    pub head_node_id: String,
    pub predicate_surface: String,
    pub normalized_predicate: String,
    #[serde(default)]
    pub dependent_node_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedded_event_node_id: Option<String>,
    pub negated: bool,
    pub evidence_surface: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticRoleGraphIR {
    pub schema: String,
    pub nodes: Vec<SemanticNodeIR>,
    pub role_edges: Vec<SemanticRoleEdgeIR>,
    pub event_relations: Vec<EventRelationEdgeIR>,
    pub quantifier_scopes: Vec<QuantifierScopeIR>,
    #[serde(default)]
    pub relative_clause_attachments: Vec<RelativeClauseAttachmentIR>,
    #[serde(default)]
    pub shared_argument_bindings: Vec<SharedArgumentBindingIR>,
    pub unresolved_roles: Vec<String>,
    pub structural_coverage_millis: u16,
}

impl Default for SemanticRoleGraphIR {
    fn default() -> Self {
        Self {
            schema: SEMANTIC_ROLE_GRAPH_SCHEMA.to_string(),
            nodes: Vec::new(),
            role_edges: Vec::new(),
            event_relations: Vec::new(),
            quantifier_scopes: Vec::new(),
            relative_clause_attachments: Vec::new(),
            shared_argument_bindings: Vec::new(),
            unresolved_roles: Vec::new(),
            structural_coverage_millis: 0,
        }
    }
}

impl SemanticRoleGraphIR {
    pub fn event_node_for_frame(&self, frame_id: &str) -> Option<&SemanticNodeIR> {
        self.nodes.iter().find(|node| {
            node.kind == SemanticNodeKindIR::Event
                && node.source_frame_id.as_deref() == Some(frame_id)
        })
    }

    pub fn arguments_for_frame(
        &self,
        frame_id: &str,
    ) -> Vec<(SemanticRoleKindIR, &SemanticNodeIR)> {
        let Some(event) = self.event_node_for_frame(frame_id) else {
            return Vec::new();
        };
        let mut arguments = self
            .role_edges
            .iter()
            .filter(|edge| edge.event_node_id == event.node_id)
            .filter_map(|edge| {
                self.nodes
                    .iter()
                    .find(|node| node.node_id == edge.argument_node_id)
                    .map(|node| (edge.role, node))
            })
            .collect::<Vec<_>>();
        arguments.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.node_id.cmp(&right.1.node_id))
        });
        arguments
    }

    pub fn primary_argument_for_frame(&self, frame_id: &str) -> Option<&SemanticNodeIR> {
        for preferred in [
            SemanticRoleKindIR::Theme,
            SemanticRoleKindIR::Patient,
            SemanticRoleKindIR::Result,
            SemanticRoleKindIR::Destination,
            SemanticRoleKindIR::Topic,
        ] {
            if let Some((_, node)) = self
                .arguments_for_frame(frame_id)
                .into_iter()
                .find(|(role, _)| *role == preferred)
            {
                return Some(node);
            }
        }
        None
    }

    pub fn role_constraint_for_frame(&self, frame_id: &str) -> Option<String> {
        let arguments = self.arguments_for_frame(frame_id);
        (!arguments.is_empty()).then(|| {
            arguments
                .into_iter()
                .map(|(role, node)| format!("{role:?}={}", node.surface))
                .collect::<Vec<_>>()
                .join(",")
        })
    }

    pub fn relative_attachment_for_frame(
        &self,
        frame_id: &str,
    ) -> Option<&RelativeClauseAttachmentIR> {
        let head = self.primary_argument_for_frame(frame_id)?;
        self.relative_clause_attachments
            .iter()
            .find(|attachment| attachment.head_node_id == head.node_id)
    }

    pub fn validate(&self) -> bool {
        if self.schema != SEMANTIC_ROLE_GRAPH_SCHEMA || self.structural_coverage_millis > 1_000 {
            return false;
        }
        let node_ids = self
            .nodes
            .iter()
            .map(|node| &node.node_id)
            .collect::<BTreeSet<_>>();
        if node_ids.len() != self.nodes.len()
            || self.nodes.iter().any(|node| {
                node.node_id.trim().is_empty()
                    || node.surface.trim().is_empty()
                    || node.source_clause_id.trim().is_empty()
            })
        {
            return false;
        }
        let attachment_ids = self
            .relative_clause_attachments
            .iter()
            .map(|attachment| &attachment.attachment_id)
            .collect::<BTreeSet<_>>();
        if attachment_ids.len() != self.relative_clause_attachments.len() {
            return false;
        }
        let binding_ids = self
            .shared_argument_bindings
            .iter()
            .map(|binding| &binding.binding_id)
            .collect::<BTreeSet<_>>();
        if binding_ids.len() != self.shared_argument_bindings.len() {
            return false;
        }
        self.role_edges.iter().all(|edge| {
            node_ids.contains(&edge.event_node_id)
                && node_ids.contains(&edge.argument_node_id)
                && edge.confidence_millis <= 1_000
        }) && self.event_relations.iter().all(|edge| {
            node_ids.contains(&edge.source_event_node_id)
                && node_ids.contains(&edge.target_event_node_id)
                && edge.confidence_millis <= 1_000
        }) && self.quantifier_scopes.iter().all(|scope| {
            node_ids.contains(&scope.target_node_id) && scope.confidence_millis <= 1_000
        }) && self.relative_clause_attachments.iter().all(|attachment| {
            !attachment.attachment_id.trim().is_empty()
                && node_ids.contains(&attachment.head_node_id)
                && !attachment.predicate_surface.trim().is_empty()
                && !attachment.normalized_predicate.trim().is_empty()
                && !attachment.evidence_surface.trim().is_empty()
                && attachment.confidence_millis <= 1_000
                && attachment.dependent_node_ids.iter().all(|node_id| {
                    node_id != &attachment.head_node_id && node_ids.contains(node_id)
                })
                && attachment
                    .embedded_event_node_id
                    .as_ref()
                    .is_none_or(|node_id| node_ids.contains(node_id))
        }) && self.shared_argument_bindings.iter().all(|binding| {
            !binding.binding_id.trim().is_empty()
                && binding.provider_event_node_id != binding.dependent_event_node_id
                && binding.confidence_millis <= 1_000
                && !binding.evidence_surface.trim().is_empty()
                && binding.syntactically_licensed
                && !binding.semantic_authority
                && !binding.external_execution_authorized
                && self.nodes.iter().any(|node| {
                    node.node_id == binding.provider_event_node_id
                        && node.kind == SemanticNodeKindIR::Event
                })
                && self.nodes.iter().any(|node| {
                    node.node_id == binding.dependent_event_node_id
                        && node.kind == SemanticNodeKindIR::Event
                })
                && self.nodes.iter().any(|node| {
                    node.node_id == binding.argument_node_id
                        && node.kind == SemanticNodeKindIR::Entity
                })
                && self.role_edges.iter().any(|edge| {
                    edge.event_node_id == binding.provider_event_node_id
                        && edge.argument_node_id == binding.argument_node_id
                        && edge.role == binding.role
                })
                && self.role_edges.iter().any(|edge| {
                    edge.event_node_id == binding.dependent_event_node_id
                        && edge.argument_node_id == binding.argument_node_id
                        && edge.role == binding.role
                })
        })
    }

    pub(crate) fn apply_clause_graph(
        &mut self,
        clause_graph: &ClauseGraphIR,
        frames: &[PredicateFrameIR],
    ) {
        let mut shareable_links = Vec::new();
        for edge in &clause_graph.edges {
            let Some(source_clause) = clause_graph
                .nodes
                .iter()
                .find(|node| node.clause_id == edge.source_clause_id)
            else {
                continue;
            };
            let Some(target_clause) = clause_graph
                .nodes
                .iter()
                .find(|node| node.clause_id == edge.target_clause_id)
            else {
                continue;
            };
            let Some(source_event) = self
                .event_node_for_frame(&source_clause.anchor_frame_id)
                .map(|node| node.node_id.clone())
            else {
                continue;
            };
            let Some(target_event) = self
                .event_node_for_frame(&target_clause.anchor_frame_id)
                .map(|node| node.node_id.clone())
            else {
                continue;
            };
            self.event_relations.retain(|relation| {
                !((relation.source_event_node_id == source_event
                    && relation.target_event_node_id == target_event)
                    || (relation.source_event_node_id == target_event
                        && relation.target_event_node_id == source_event))
            });
            self.event_relations.push(EventRelationEdgeIR {
                source_event_node_id: source_event.clone(),
                target_event_node_id: target_event.clone(),
                relation: semantic_event_relation(edge.relation),
                evidence_surface: edge.marker_surface.clone(),
                confidence_millis: edge.confidence_millis,
            });

            if relation_licenses_argument_sharing(edge.relation) {
                let source_start = frames
                    .iter()
                    .find(|frame| frame.frame_id == source_clause.anchor_frame_id)
                    .map(|frame| frame.source_start_byte)
                    .unwrap_or(source_clause.source_start_byte);
                let target_start = frames
                    .iter()
                    .find(|frame| frame.frame_id == target_clause.anchor_frame_id)
                    .map(|frame| frame.source_start_byte)
                    .unwrap_or(target_clause.source_start_byte);
                let (left_event, right_event) = if source_start <= target_start {
                    (source_event, target_event)
                } else {
                    (target_event, source_event)
                };
                shareable_links.push(ShareableClauseLink {
                    left_event,
                    right_event,
                    relation: edge.relation,
                    evidence_surface: edge.marker_surface.clone(),
                    confidence_millis: edge.confidence_millis.min(900),
                });
            }
        }
        self.event_relations.sort_by(|left, right| {
            left.source_event_node_id
                .cmp(&right.source_event_node_id)
                .then_with(|| left.target_event_node_id.cmp(&right.target_event_node_id))
                .then_with(|| left.relation.cmp(&right.relation))
        });
        self.event_relations.dedup_by(|left, right| {
            left.source_event_node_id == right.source_event_node_id
                && left.target_event_node_id == right.target_event_node_id
                && left.relation == right.relation
        });
        for _ in 0..shareable_links.len().max(1) {
            let mut changed = false;
            for link in shareable_links.iter().rev() {
                changed |= self.propagate_shared_theme(link);
            }
            for link in &shareable_links {
                changed |= self.propagate_shared_theme(link);
            }
            if !changed {
                break;
            }
        }
        self.remove_orphan_argument_nodes();
        self.shared_argument_bindings.sort_by(|left, right| {
            left.dependent_event_node_id
                .cmp(&right.dependent_event_node_id)
                .then_with(|| {
                    left.provider_event_node_id
                        .cmp(&right.provider_event_node_id)
                })
                .then_with(|| left.role.cmp(&right.role))
        });
        for (index, binding) in self.shared_argument_bindings.iter_mut().enumerate() {
            binding.binding_id = format!("SHARED-ARG-{:03}", index + 1);
        }
        let resolved_theme_frames = self
            .nodes
            .iter()
            .filter(|node| node.kind == SemanticNodeKindIR::Event)
            .filter(|node| {
                self.role_edges.iter().any(|edge| {
                    edge.event_node_id == node.node_id
                        && matches!(
                            edge.role,
                            SemanticRoleKindIR::Theme | SemanticRoleKindIR::Patient
                        )
                })
            })
            .filter_map(|node| node.source_frame_id.clone())
            .collect::<BTreeSet<_>>();
        self.unresolved_roles.retain(|unresolved| {
            unresolved
                .strip_suffix(":THEME")
                .is_none_or(|frame_id| !resolved_theme_frames.contains(frame_id))
        });
        debug_assert!(frames
            .iter()
            .all(|frame| self.event_node_for_frame(&frame.frame_id).is_some()));
        debug_assert!(self.validate());
    }

    fn propagate_shared_theme(&mut self, link: &ShareableClauseLink) -> bool {
        let left_theme = best_shareable_theme(&self.role_edges, &link.left_event);
        let right_theme = best_shareable_theme(&self.role_edges, &link.right_event);
        let (provider, dependent, direction) = match (left_theme, right_theme) {
            (Some(_), None)
                if has_resolved_primary_argument(
                    &self.role_edges,
                    &link.right_event,
                    link.relation,
                ) =>
            {
                return false;
            }
            (None, Some(_))
                if has_resolved_primary_argument(
                    &self.role_edges,
                    &link.left_event,
                    link.relation,
                ) =>
            {
                return false;
            }
            (Some(provider), None) => (
                provider,
                link.right_event.as_str(),
                SharedArgumentDirectionIR::Forward,
            ),
            (None, Some(provider)) => (
                provider,
                link.left_event.as_str(),
                SharedArgumentDirectionIR::Backward,
            ),
            _ => return false,
        };
        let provider_event = provider.event_node_id.clone();
        let argument_node = provider.argument_node_id.clone();
        let role = provider.role;
        let coordinated_arguments = self
            .role_edges
            .iter()
            .filter(|edge| {
                edge.event_node_id == provider_event
                    && edge.argument_node_id != argument_node
                    && edge.role == SemanticRoleKindIR::CoTheme
            })
            .cloned()
            .collect::<Vec<_>>();
        if !replace_fallback_theme(
            &mut self.role_edges,
            dependent,
            &provider,
            link.relation == ClauseRelationKindIR::Sequence,
        ) {
            return false;
        }
        self.shared_argument_bindings.push(SharedArgumentBindingIR {
            binding_id: String::new(),
            provider_event_node_id: provider_event.clone(),
            dependent_event_node_id: dependent.to_string(),
            argument_node_id: argument_node,
            role,
            direction,
            relation: link.relation,
            evidence_surface: link.evidence_surface.clone(),
            confidence_millis: link.confidence_millis,
            syntactically_licensed: true,
            semantic_authority: false,
            external_execution_authorized: false,
        });
        for coordinated in coordinated_arguments {
            if self.role_edges.iter().any(|edge| {
                edge.event_node_id == dependent
                    && edge.argument_node_id == coordinated.argument_node_id
                    && edge.role == SemanticRoleKindIR::CoTheme
            }) {
                continue;
            }
            self.role_edges.push(SemanticRoleEdgeIR {
                event_node_id: dependent.to_string(),
                argument_node_id: coordinated.argument_node_id.clone(),
                role: SemanticRoleKindIR::CoTheme,
                evidence_surface: "typed shared coordinated argument binding".to_string(),
                confidence_millis: 880,
            });
            self.shared_argument_bindings.push(SharedArgumentBindingIR {
                binding_id: String::new(),
                provider_event_node_id: provider_event.clone(),
                dependent_event_node_id: dependent.to_string(),
                argument_node_id: coordinated.argument_node_id,
                role: SemanticRoleKindIR::CoTheme,
                direction,
                relation: link.relation,
                evidence_surface: link.evidence_surface.clone(),
                confidence_millis: link.confidence_millis,
                syntactically_licensed: true,
                semantic_authority: false,
                external_execution_authorized: false,
            });
        }
        true
    }

    fn remove_orphan_argument_nodes(&mut self) {
        let mut referenced = self
            .role_edges
            .iter()
            .map(|edge| edge.argument_node_id.clone())
            .collect::<BTreeSet<_>>();
        for attachment in &self.relative_clause_attachments {
            referenced.insert(attachment.head_node_id.clone());
            referenced.extend(attachment.dependent_node_ids.iter().cloned());
        }
        self.nodes.retain(|node| {
            node.kind != SemanticNodeKindIR::Entity || referenced.contains(&node.node_id)
        });
        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.node_id.clone())
            .collect::<BTreeSet<_>>();
        self.quantifier_scopes
            .retain(|scope| node_ids.contains(&scope.target_node_id));
    }
}

#[derive(Debug, Clone)]
struct ShareableClauseLink {
    left_event: String,
    right_event: String,
    relation: ClauseRelationKindIR,
    evidence_surface: String,
    confidence_millis: u16,
}

fn relation_licenses_argument_sharing(relation: ClauseRelationKindIR) -> bool {
    matches!(
        relation,
        ClauseRelationKindIR::Coordination
            | ClauseRelationKindIR::Sequence
            | ClauseRelationKindIR::TemporalBefore
            | ClauseRelationKindIR::Contrast
    )
}

fn semantic_event_relation(relation: ClauseRelationKindIR) -> EventRelationKindIR {
    match relation {
        ClauseRelationKindIR::Coordination => EventRelationKindIR::Coordination,
        ClauseRelationKindIR::Sequence | ClauseRelationKindIR::TemporalBefore => {
            EventRelationKindIR::TemporalBefore
        }
        ClauseRelationKindIR::Condition => EventRelationKindIR::Condition,
        ClauseRelationKindIR::Cause => EventRelationKindIR::Cause,
        ClauseRelationKindIR::Purpose => EventRelationKindIR::Purpose,
        ClauseRelationKindIR::Contrast => EventRelationKindIR::Contrast,
    }
}

fn best_shareable_theme(
    edges: &[SemanticRoleEdgeIR],
    event_node_id: &str,
) -> Option<SemanticRoleEdgeIR> {
    edges
        .iter()
        .filter(|edge| {
            edge.event_node_id == event_node_id
                && matches!(
                    edge.role,
                    SemanticRoleKindIR::Theme | SemanticRoleKindIR::Patient
                )
                && edge.evidence_surface != "predicate theme fallback"
        })
        .max_by_key(|edge| edge.confidence_millis)
        .cloned()
}

fn has_resolved_primary_argument(
    edges: &[SemanticRoleEdgeIR],
    event_node_id: &str,
    relation: ClauseRelationKindIR,
) -> bool {
    edges.iter().any(|edge| {
        (edge.event_node_id == event_node_id
            && edge.evidence_surface != "predicate theme fallback"
            && matches!(
                edge.role,
                SemanticRoleKindIR::Theme
                    | SemanticRoleKindIR::Patient
                    | SemanticRoleKindIR::Result
                    | SemanticRoleKindIR::Destination
                    | SemanticRoleKindIR::Topic
            ))
            || (edge.event_node_id == event_node_id
                && edge.role == SemanticRoleKindIR::PriorResult
                && relation != ClauseRelationKindIR::Sequence)
    })
}

fn replace_fallback_theme(
    edges: &mut Vec<SemanticRoleEdgeIR>,
    event_node_id: &str,
    provider: &SemanticRoleEdgeIR,
    replace_sequence_prior_result: bool,
) -> bool {
    edges.retain(|edge| {
        !(edge.event_node_id == event_node_id
            && ((matches!(
                edge.role,
                SemanticRoleKindIR::Theme | SemanticRoleKindIR::Patient
            ) && edge.confidence_millis <= 700)
                || (replace_sequence_prior_result && edge.role == SemanticRoleKindIR::PriorResult)))
    });
    if edges.iter().any(|edge| {
        edge.event_node_id == event_node_id
            && edge.argument_node_id == provider.argument_node_id
            && matches!(
                edge.role,
                SemanticRoleKindIR::Theme | SemanticRoleKindIR::Patient
            )
    }) {
        return false;
    }
    edges.push(SemanticRoleEdgeIR {
        event_node_id: event_node_id.to_string(),
        argument_node_id: provider.argument_node_id.clone(),
        role: provider.role,
        evidence_surface: "typed shared argument binding".to_string(),
        confidence_millis: 880,
    });
    true
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SemanticRoleAnalyzer;

impl SemanticRoleAnalyzer {
    pub fn analyze(&self, text: &str, frames: &[PredicateFrameIR]) -> SemanticRoleGraphIR {
        if frames.is_empty() {
            return SemanticRoleGraphIR::default();
        }
        let mut builder = GraphBuilder::default();
        let mut ordered = frames.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|frame| frame.source_start_byte);
        for (index, frame) in ordered.iter().enumerate() {
            let event_node_id = builder.add_event(frame);
            let (default_start, default_end) = clause_bounds(text, frame.source_start_byte);
            let (clause_start, clause_end) = if frame.embedded_under_quote {
                enclosing_quote_bounds(text, frame.source_start_byte)
                    .unwrap_or((default_start, default_end))
            } else {
                (default_start, default_end)
            };
            let prior_end = index
                .checked_sub(1)
                .and_then(|prior| ordered.get(prior))
                .filter(|prior| {
                    prior.clause_id == frame.clause_id
                        && prior.embedded_under_quote == frame.embedded_under_quote
                        && (prior.mood == FrameMoodIR::RelativeClause)
                            == (frame.mood == FrameMoodIR::RelativeClause)
                        && prior.source_start_byte >= clause_start
                })
                .map_or(clause_start, |prior| {
                    prior.source_start_byte + prior.predicate_surface.len()
                });
            let next_start = ordered
                .get(index + 1)
                .filter(|next| {
                    next.clause_id == frame.clause_id
                        && next.embedded_under_quote == frame.embedded_under_quote
                        && (next.mood == FrameMoodIR::RelativeClause)
                            == (frame.mood == FrameMoodIR::RelativeClause)
                        && next.source_start_byte < clause_end
                })
                .map_or(clause_end, |next| next.source_start_byte);
            let before_source = safe_slice(text, prior_end, frame.source_start_byte);
            let after_start = frame.source_start_byte + frame.predicate_surface.len();
            let after_source = safe_slice(text, after_start, next_start);
            let clause_source = safe_slice(text, clause_start, clause_end);
            let before_visible;
            let after_visible;
            let clause_visible;
            let (before, after, clause) = if frame.embedded_under_quote {
                (before_source, after_source, clause_source)
            } else {
                before_visible = remove_quoted_content(before_source);
                after_visible = remove_quoted_content(after_source);
                clause_visible = remove_quoted_content(clause_source);
                (
                    before_visible.as_str(),
                    after_visible.as_str(),
                    clause_visible.as_str(),
                )
            };
            if contains_hangul(clause) {
                extract_korean_arguments(
                    &mut builder,
                    &event_node_id,
                    frame,
                    before,
                    after,
                    clause,
                );
            } else {
                extract_english_arguments(
                    &mut builder,
                    &event_node_id,
                    frame,
                    before,
                    after,
                    clause,
                );
            }
            if frame.theme == "PRIOR_RESULT" {
                if let Some(prior) = index.checked_sub(1).and_then(|prior| ordered.get(prior)) {
                    if let Some(prior_event) = builder.frame_events.get(&prior.frame_id).cloned() {
                        builder.add_role(
                            &event_node_id,
                            &prior_event,
                            SemanticRoleKindIR::PriorResult,
                            "prior event result",
                            930,
                        );
                    }
                }
            }
            builder.ensure_required_roles(&event_node_id, frame);
        }
        infer_relative_clause_attachments(text, &ordered, &mut builder);
        let relation_frames = ordered
            .iter()
            .copied()
            .filter(|frame| frame.mood != FrameMoodIR::RelativeClause)
            .collect::<Vec<_>>();
        infer_event_relations(text, &relation_frames, &mut builder);
        builder.finish(frames.len())
    }
}

#[derive(Debug, Clone)]
struct WordSpan<'a> {
    text: &'a str,
    start_byte: usize,
    end_byte: usize,
}

#[derive(Debug, Clone)]
struct RelativePhraseParts {
    head_surface: String,
    predicate_surface: String,
    normalized_predicate: String,
    dependent_surface: Option<String>,
    negated: bool,
    evidence_surface: String,
}

fn english_relative_phrase(surface: &str) -> Option<RelativePhraseParts> {
    let lower = surface.to_lowercase();
    let (marker_start, marker_len) = [" that ", " which "]
        .into_iter()
        .filter_map(|marker| lower.find(marker).map(|start| (start, marker.len())))
        .min_by_key(|(start, _)| *start)?;
    let head_surface = surface[..marker_start].trim().to_string();
    let tail = surface[marker_start + marker_len..].trim();
    let words = tail.split_whitespace().collect::<Vec<_>>();
    if head_surface.is_empty() || words.is_empty() {
        return None;
    }
    let first = clean_relative_word(words[0]);
    let link_first = matches!(
        first.as_str(),
        "is" | "are"
            | "was"
            | "were"
            | "has"
            | "have"
            | "had"
            | "contain"
            | "contains"
            | "include"
            | "includes"
            | "lack"
            | "lacks"
    );
    let predicate_index = if link_first { 0 } else { words.len() - 1 };
    let predicate_surface = clean_relative_word(words[predicate_index]);
    if predicate_surface.is_empty() {
        return None;
    }
    let dependent_words = if predicate_index == 0 {
        &words[1..]
    } else {
        &words[..predicate_index]
    };
    let dependent_surface = (!dependent_words.is_empty())
        .then(|| dependent_words.join(" "))
        .filter(|value| !normalize_argument(value).is_empty());
    let negated = words
        .iter()
        .any(|word| matches!(clean_relative_word(word).as_str(), "not" | "no" | "never"));
    Some(RelativePhraseParts {
        head_surface,
        predicate_surface: predicate_surface.clone(),
        normalized_predicate: normalize_relative_predicate(&predicate_surface),
        dependent_surface,
        negated,
        evidence_surface: surface.trim().to_string(),
    })
}

fn clean_relative_word(word: &str) -> String {
    word.trim_matches(|character: char| !character.is_alphanumeric() && character != '_')
        .to_lowercase()
}

fn normalize_relative_predicate(predicate: &str) -> String {
    match predicate {
        "is" | "are" | "was" | "were" => "STATE".to_string(),
        "has" | "have" | "had" => "HAS".to_string(),
        "contain" | "contains" => "CONTAINS".to_string(),
        "include" | "includes" => "INCLUDES".to_string(),
        "lack" | "lacks" => "LACKS".to_string(),
        _ => predicate
            .trim_end_matches("ed")
            .trim_end_matches("ing")
            .to_uppercase(),
    }
}

#[derive(Default)]
struct GraphBuilder {
    nodes: Vec<SemanticNodeIR>,
    role_edges: Vec<SemanticRoleEdgeIR>,
    event_relations: Vec<EventRelationEdgeIR>,
    quantifier_scopes: Vec<QuantifierScopeIR>,
    relative_clause_attachments: Vec<RelativeClauseAttachmentIR>,
    unresolved_roles: BTreeSet<String>,
    frame_events: BTreeMap<String, String>,
    next_argument: usize,
}

impl GraphBuilder {
    fn add_event(&mut self, frame: &PredicateFrameIR) -> String {
        let node_id = format!("EVENT-{}", frame.frame_id);
        self.nodes.push(SemanticNodeIR {
            node_id: node_id.clone(),
            kind: SemanticNodeKindIR::Event,
            surface: frame.predicate_surface.clone(),
            normalized_label: frame.canonical_predicate.clone(),
            concept_id_hint: None,
            source_frame_id: Some(frame.frame_id.clone()),
            source_clause_id: frame.clause_id.clone(),
        });
        self.frame_events
            .insert(frame.frame_id.clone(), node_id.clone());
        node_id
    }

    fn add_argument(
        &mut self,
        event_node_id: &str,
        clause_id: &str,
        surface: &str,
        role: SemanticRoleKindIR,
        evidence: &str,
        confidence_millis: u16,
    ) -> Option<String> {
        if !contains_hangul(surface)
            && matches!(
                role,
                SemanticRoleKindIR::Theme
                    | SemanticRoleKindIR::Patient
                    | SemanticRoleKindIR::Topic
                    | SemanticRoleKindIR::Result
            )
        {
            if let Some(relative) = english_relative_phrase(surface) {
                return self.add_relative_argument(
                    event_node_id,
                    clause_id,
                    role,
                    evidence,
                    confidence_millis,
                    relative,
                );
            }
        }
        self.add_argument_plain(
            event_node_id,
            clause_id,
            surface,
            role,
            evidence,
            confidence_millis,
        )
    }

    fn add_argument_plain(
        &mut self,
        event_node_id: &str,
        clause_id: &str,
        surface: &str,
        role: SemanticRoleKindIR,
        evidence: &str,
        confidence_millis: u16,
    ) -> Option<String> {
        let normalized = normalize_argument(surface);
        if normalized.is_empty() || is_non_argument(&normalized) {
            return None;
        }
        if let Some(edge) = self.role_edges.iter().find(|edge| {
            edge.event_node_id == event_node_id
                && edge.role == role
                && self.nodes.iter().any(|node| {
                    node.node_id == edge.argument_node_id && node.normalized_label == normalized
                })
        }) {
            return Some(edge.argument_node_id.clone());
        }
        self.next_argument += 1;
        let node_id = format!("ARG-{:03}", self.next_argument);
        self.nodes.push(SemanticNodeIR {
            node_id: node_id.clone(),
            kind: SemanticNodeKindIR::Entity,
            surface: surface.trim().to_string(),
            normalized_label: normalized.clone(),
            concept_id_hint: concept_hint(&normalized).map(ToString::to_string),
            source_frame_id: None,
            source_clause_id: clause_id.to_string(),
        });
        self.add_role(event_node_id, &node_id, role, evidence, confidence_millis);
        self.attach_quantifier(&node_id, surface, 900);
        Some(node_id)
    }

    fn add_relative_argument(
        &mut self,
        event_node_id: &str,
        clause_id: &str,
        role: SemanticRoleKindIR,
        evidence: &str,
        confidence_millis: u16,
        relative: RelativePhraseParts,
    ) -> Option<String> {
        let head_node_id = self.add_argument_plain(
            event_node_id,
            clause_id,
            &relative.head_surface,
            role,
            evidence,
            confidence_millis,
        )?;
        let dependent_node_ids = relative
            .dependent_surface
            .as_deref()
            .and_then(|surface| self.add_detached_entity(clause_id, surface))
            .into_iter()
            .collect::<Vec<_>>();
        self.relative_clause_attachments
            .push(RelativeClauseAttachmentIR {
                attachment_id: String::new(),
                head_node_id: head_node_id.clone(),
                predicate_surface: relative.predicate_surface.clone(),
                normalized_predicate: relative.normalized_predicate,
                dependent_node_ids,
                embedded_event_node_id: None,
                negated: relative.negated,
                evidence_surface: relative.evidence_surface,
                confidence_millis: 930,
            });
        Some(head_node_id)
    }

    fn add_detached_entity(&mut self, clause_id: &str, surface: &str) -> Option<String> {
        let normalized = normalize_argument(surface);
        if normalized.is_empty() || is_non_argument(&normalized) {
            return None;
        }
        self.next_argument += 1;
        let node_id = format!("ARG-{:03}", self.next_argument);
        self.nodes.push(SemanticNodeIR {
            node_id: node_id.clone(),
            kind: SemanticNodeKindIR::Entity,
            surface: surface.trim().to_string(),
            normalized_label: normalized.clone(),
            concept_id_hint: concept_hint(&normalized).map(ToString::to_string),
            source_frame_id: None,
            source_clause_id: clause_id.to_string(),
        });
        self.attach_quantifier(&node_id, surface, 900);
        Some(node_id)
    }

    fn attach_quantifier(&mut self, node_id: &str, surface: &str, confidence_millis: u16) {
        if let Some((quantifier, cardinality, negated, quantifier_evidence)) =
            detect_quantifier(surface)
        {
            self.quantifier_scopes.push(QuantifierScopeIR {
                target_node_id: node_id.to_string(),
                quantifier,
                cardinality,
                negated,
                evidence_surface: quantifier_evidence,
                confidence_millis,
            });
        }
    }

    fn add_implicit_user(&mut self, event_node_id: &str, clause_id: &str) {
        let node_id = format!("IMPLICIT-USER-{event_node_id}");
        if !self.nodes.iter().any(|node| node.node_id == node_id) {
            self.nodes.push(SemanticNodeIR {
                node_id: node_id.clone(),
                kind: SemanticNodeKindIR::ImplicitAgent,
                surface: "USER".to_string(),
                normalized_label: "dialogue_speaker".to_string(),
                concept_id_hint: Some("C_DIALOGUE_SPEAKER".to_string()),
                source_frame_id: None,
                source_clause_id: clause_id.to_string(),
            });
        }
        self.add_role(
            event_node_id,
            &node_id,
            SemanticRoleKindIR::Agent,
            "imperative subject is the dialogue speaker",
            980,
        );
    }

    fn add_role(
        &mut self,
        event_node_id: &str,
        argument_node_id: &str,
        role: SemanticRoleKindIR,
        evidence: &str,
        confidence_millis: u16,
    ) {
        if !self.role_edges.iter().any(|edge| {
            edge.event_node_id == event_node_id
                && edge.argument_node_id == argument_node_id
                && edge.role == role
        }) {
            self.role_edges.push(SemanticRoleEdgeIR {
                event_node_id: event_node_id.to_string(),
                argument_node_id: argument_node_id.to_string(),
                role,
                evidence_surface: evidence.to_string(),
                confidence_millis,
            });
        }
    }

    fn ensure_required_roles(&mut self, event_node_id: &str, frame: &PredicateFrameIR) {
        let has_agent = self.role_edges.iter().any(|edge| {
            edge.event_node_id == event_node_id && edge.role == SemanticRoleKindIR::Agent
        });
        if !has_agent && frame.mood == FrameMoodIR::Imperative {
            self.add_implicit_user(event_node_id, &frame.clause_id);
        }
        let has_theme = self.role_edges.iter().any(|edge| {
            edge.event_node_id == event_node_id
                && matches!(
                    edge.role,
                    SemanticRoleKindIR::Theme
                        | SemanticRoleKindIR::Patient
                        | SemanticRoleKindIR::PriorResult
                        | SemanticRoleKindIR::Result
                )
        });
        if !has_theme
            && matches!(
                frame.intent_hint,
                PlanIntentIR::Repair | PlanIntentIR::Create | PlanIntentIR::Execute
            )
        {
            self.unresolved_roles
                .insert(format!("{}:THEME", frame.frame_id));
        }
    }

    fn finish(mut self, frame_count: usize) -> SemanticRoleGraphIR {
        self.nodes
            .sort_by(|left, right| left.node_id.cmp(&right.node_id));
        self.role_edges.sort_by(|left, right| {
            left.event_node_id
                .cmp(&right.event_node_id)
                .then_with(|| left.role.cmp(&right.role))
                .then_with(|| left.argument_node_id.cmp(&right.argument_node_id))
        });
        self.event_relations.sort_by(|left, right| {
            left.source_event_node_id
                .cmp(&right.source_event_node_id)
                .then_with(|| left.target_event_node_id.cmp(&right.target_event_node_id))
                .then_with(|| left.relation.cmp(&right.relation))
        });
        self.event_relations.dedup();
        self.quantifier_scopes.sort_by(|left, right| {
            left.target_node_id
                .cmp(&right.target_node_id)
                .then_with(|| left.quantifier.cmp(&right.quantifier))
        });
        self.relative_clause_attachments.sort_by(|left, right| {
            left.head_node_id
                .cmp(&right.head_node_id)
                .then_with(|| left.attachment_id.cmp(&right.attachment_id))
        });
        self.relative_clause_attachments.dedup_by(|left, right| {
            left.head_node_id == right.head_node_id
                && left.predicate_surface == right.predicate_surface
                && left.dependent_node_ids == right.dependent_node_ids
        });
        for (index, attachment) in self.relative_clause_attachments.iter_mut().enumerate() {
            attachment.attachment_id = format!("REL-ATTACH-{:03}", index + 1);
        }
        let resolved_events = self
            .nodes
            .iter()
            .filter(|node| node.kind == SemanticNodeKindIR::Event)
            .filter(|event| {
                self.role_edges
                    .iter()
                    .any(|edge| edge.event_node_id == event.node_id)
            })
            .count();
        let structural_coverage_millis = resolved_events
            .saturating_mul(1_000)
            .checked_div(frame_count)
            .and_then(|coverage| u16::try_from(coverage).ok())
            .unwrap_or_default();
        let graph = SemanticRoleGraphIR {
            schema: SEMANTIC_ROLE_GRAPH_SCHEMA.to_string(),
            nodes: self.nodes,
            role_edges: self.role_edges,
            event_relations: self.event_relations,
            quantifier_scopes: self.quantifier_scopes,
            relative_clause_attachments: self.relative_clause_attachments,
            shared_argument_bindings: Vec::new(),
            unresolved_roles: self.unresolved_roles.into_iter().collect(),
            structural_coverage_millis,
        };
        debug_assert!(graph.validate());
        graph
    }
}

fn extract_korean_arguments(
    builder: &mut GraphBuilder,
    event_node_id: &str,
    frame: &PredicateFrameIR,
    before: &str,
    after: &str,
    clause: &str,
) {
    let words = word_spans(before);
    for (index, word) in words.iter().enumerate() {
        let Some((base, particle)) = strip_korean_particle(word.text) else {
            continue;
        };
        let role = korean_particle_role(particle, frame);
        let phrase = korean_phrase(&words, index, base);
        builder.add_argument(
            event_node_id,
            &frame.clause_id,
            &phrase,
            role,
            particle,
            920,
        );
    }
    for (index, word) in word_spans(after).iter().enumerate() {
        if let Some((base, particle)) = strip_korean_particle(word.text) {
            let phrase = korean_phrase(&word_spans(after), index, base);
            builder.add_argument(
                event_node_id,
                &frame.clause_id,
                &phrase,
                korean_particle_role(particle, frame),
                particle,
                880,
            );
        }
    }
    if !builder.role_edges.iter().any(|edge| {
        edge.event_node_id == event_node_id
            && matches!(
                edge.role,
                SemanticRoleKindIR::Theme | SemanticRoleKindIR::Patient
            )
    }) && !frame.theme.is_empty()
        && frame.theme != "PRIOR_RESULT"
    {
        builder.add_argument(
            event_node_id,
            &frame.clause_id,
            &frame.theme,
            SemanticRoleKindIR::Theme,
            "predicate theme fallback",
            700,
        );
    }
    attach_clause_quantifier_fallback(builder, event_node_id, clause);
}

fn extract_english_arguments(
    builder: &mut GraphBuilder,
    event_node_id: &str,
    frame: &PredicateFrameIR,
    before: &str,
    after: &str,
    clause: &str,
) {
    let before_spans = word_spans(before);
    let before_words = before_spans
        .iter()
        .map(|word| word.text)
        .collect::<Vec<_>>();
    let after_spans = word_spans(after);
    let after_words = after_spans.iter().map(|word| word.text).collect::<Vec<_>>();
    let passive_aux = before_words.iter().rposition(|word| {
        matches!(
            word.to_lowercase().as_str(),
            "is" | "are" | "was" | "were" | "be" | "been" | "being"
        )
    });
    if let Some(auxiliary) = passive_aux {
        let theme = word_span_surface(before, &before_spans, 0, auxiliary);
        add_english_argument_group(
            builder,
            event_node_id,
            &frame.clause_id,
            theme,
            SemanticRoleKindIR::Patient,
            "passive subject before auxiliary",
            930,
        );
    } else if frame.mood != FrameMoodIR::Imperative && !before_words.is_empty() {
        let agent = clean_english_subject(&before_words);
        builder.add_argument(
            event_node_id,
            &frame.clause_id,
            &agent,
            SemanticRoleKindIR::Agent,
            "pre-predicate subject",
            820,
        );
    }
    let direct_end = english_direct_argument_end(&after_words);
    let direct = word_span_surface(after, &after_spans, 0, direct_end);
    if !direct.trim().is_empty() {
        add_english_argument_group(
            builder,
            event_node_id,
            &frame.clause_id,
            direct,
            SemanticRoleKindIR::Theme,
            "direct post-predicate argument",
            900,
        );
    }
    let mut index = direct_end;
    while index < after_words.len() {
        let Some(preposition) = english_preposition_at(&after_words, index, after_words[index])
        else {
            index += 1;
            continue;
        };
        let start = index + 1;
        let end = after_words[start..]
            .iter()
            .enumerate()
            .position(|(offset, word)| {
                english_preposition_at(&after_words, start + offset, word).is_some()
            })
            .map_or(after_words.len(), |offset| start + offset);
        let phrase = word_span_surface(after, &after_spans, start, end);
        let role = english_preposition_role(preposition, frame);
        add_english_argument_group(
            builder,
            event_node_id,
            &frame.clause_id,
            phrase,
            role,
            preposition,
            900,
        );
        index = end;
    }
    if passive_aux.is_some() {
        if let Some(by_index) = after_words
            .iter()
            .position(|word| word.eq_ignore_ascii_case("by"))
        {
            let agent = word_span_surface(after, &after_spans, by_index + 1, after_spans.len());
            add_english_argument_group(
                builder,
                event_node_id,
                &frame.clause_id,
                agent,
                SemanticRoleKindIR::Agent,
                "by",
                950,
            );
        }
    }
    if !builder.role_edges.iter().any(|edge| {
        edge.event_node_id == event_node_id
            && matches!(
                edge.role,
                SemanticRoleKindIR::Theme | SemanticRoleKindIR::Patient
            )
    }) && !frame.theme.is_empty()
        && frame.theme != "PRIOR_RESULT"
    {
        builder.add_argument(
            event_node_id,
            &frame.clause_id,
            &frame.theme,
            SemanticRoleKindIR::Theme,
            "predicate theme fallback",
            700,
        );
    }
    attach_clause_quantifier_fallback(builder, event_node_id, clause);
}

fn add_english_argument_group(
    builder: &mut GraphBuilder,
    event_node_id: &str,
    clause_id: &str,
    surface: &str,
    primary_role: SemanticRoleKindIR,
    evidence: &str,
    confidence_millis: u16,
) {
    let parts = split_english_coordinated_argument(surface);
    let coordinated = parts.len() > 1;
    for (index, part) in parts.iter().enumerate() {
        let role = if coordinated
            && index > 0
            && matches!(
                primary_role,
                SemanticRoleKindIR::Theme
                    | SemanticRoleKindIR::Patient
                    | SemanticRoleKindIR::Topic
                    | SemanticRoleKindIR::Result
            ) {
            SemanticRoleKindIR::CoTheme
        } else {
            primary_role
        };
        let member_evidence = if coordinated {
            format!("{evidence}; coordinated argument")
        } else {
            evidence.to_string()
        };
        builder.add_argument(
            event_node_id,
            clause_id,
            part,
            role,
            &member_evidence,
            confidence_millis,
        );
    }
}

fn english_direct_argument_end(words: &[&str]) -> usize {
    words
        .iter()
        .enumerate()
        .position(|(index, word)| {
            if english_preposition_at(words, index, word).is_some() {
                return true;
            }
            let lower = word.to_lowercase();
            if matches!(
                lower.as_str(),
                "then" | "but" | "because" | "if" | "unless" | "when" | "once"
            ) {
                return true;
            }
            lower == "and"
                && words.get(index + 1).is_some_and(|next| {
                    matches!(
                        next.to_lowercase().as_str(),
                        "if" | "unless"
                            | "when"
                            | "once"
                            | "because"
                            | "do"
                            | "does"
                            | "did"
                            | "not"
                            | "never"
                    )
                })
        })
        .unwrap_or(words.len())
}

fn split_english_coordinated_argument(surface: &str) -> Vec<String> {
    let trimmed = strip_trailing_english_coordinator(surface.trim());
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.contains(',') {
        let mut parts = Vec::new();
        for comma_part in trimmed.split(',') {
            let part = comma_part.trim();
            if part.is_empty() {
                continue;
            }
            let part = strip_leading_english_coordinator(part);
            let nested = split_simple_english_conjunction(part);
            parts.extend(nested);
        }
        if parts.len() > 1 {
            return parts;
        }
    }
    split_simple_english_conjunction(trimmed)
}

fn split_simple_english_conjunction(surface: &str) -> Vec<String> {
    let lower = surface.to_lowercase();
    let Some(position) = lower.rfind(" and ") else {
        return vec![surface.trim().to_string()];
    };
    let left = surface[..position].trim();
    let right = surface[position + " and ".len()..].trim();
    if left.is_empty() || right.is_empty() {
        return vec![surface.trim().to_string()];
    }
    let right_starts_member_marker = right
        .split_whitespace()
        .next()
        .is_some_and(is_english_argument_member_marker);
    let left_content_words = english_argument_content_word_count(left);
    let right_content_words = english_argument_content_word_count(right);
    let separately_marked =
        right_starts_member_marker && left_content_words > 0 && right_content_words > 0;
    let both_simple = left_content_words == 1 && right_content_words == 1;
    if !separately_marked && !both_simple {
        return vec![surface.trim().to_string()];
    }
    vec![left.to_string(), right.to_string()]
}

fn strip_leading_english_coordinator(surface: &str) -> &str {
    let trimmed = surface.trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("and ") {
        trimmed["and ".len()..].trim()
    } else {
        trimmed
    }
}

fn strip_trailing_english_coordinator(surface: &str) -> &str {
    let trimmed = surface.trim();
    let lower = trimmed.to_lowercase();
    if lower.ends_with(" and") {
        trimmed[..trimmed.len() - " and".len()].trim()
    } else {
        trimmed
    }
}

fn is_english_argument_member_marker(word: &str) -> bool {
    matches!(
        word.trim_matches(|character: char| !character.is_ascii_alphanumeric())
            .to_lowercase()
            .as_str(),
        "the" | "a" | "an" | "all" | "every" | "each" | "some" | "any" | "no"
    )
}

fn english_argument_content_word_count(surface: &str) -> usize {
    surface
        .split_whitespace()
        .filter(|word| !is_english_argument_member_marker(word))
        .count()
}

fn word_span_surface<'a>(
    source: &'a str,
    spans: &[WordSpan<'_>],
    start: usize,
    end: usize,
) -> &'a str {
    if start >= end || end > spans.len() {
        return "";
    }
    safe_slice(source, spans[start].start_byte, spans[end - 1].end_byte)
}

fn infer_relative_clause_attachments(
    text: &str,
    ordered: &[&PredicateFrameIR],
    builder: &mut GraphBuilder,
) {
    for index in 0..builder.relative_clause_attachments.len() {
        let evidence = builder.relative_clause_attachments[index]
            .evidence_surface
            .to_lowercase();
        if let Some(frame) = ordered.iter().copied().find(|frame| {
            frame.mood == FrameMoodIR::RelativeClause
                && evidence.contains(&frame.predicate_surface.to_lowercase())
        }) {
            builder.relative_clause_attachments[index].normalized_predicate =
                frame.canonical_predicate.clone();
            builder.relative_clause_attachments[index].embedded_event_node_id =
                builder.frame_events.get(&frame.frame_id).cloned();
        }
    }

    for outer in ordered
        .iter()
        .copied()
        .filter(|frame| frame.mood != FrameMoodIR::RelativeClause)
    {
        let Some(head_node_id) = primary_argument_id(builder, &outer.frame_id) else {
            continue;
        };
        if builder
            .relative_clause_attachments
            .iter()
            .any(|attachment| attachment.head_node_id == head_node_id)
        {
            continue;
        }
        let clause_start = clause_bounds(text, outer.source_start_byte).0;
        let prefix = safe_slice(text, clause_start, outer.source_start_byte);
        if !contains_hangul(prefix) {
            continue;
        }
        let embedded = ordered
            .iter()
            .copied()
            .filter(|frame| {
                frame.clause_id == outer.clause_id
                    && frame.mood == FrameMoodIR::RelativeClause
                    && frame.source_start_byte < outer.source_start_byte
            })
            .max_by_key(|frame| frame.source_start_byte);
        let predicate = embedded
            .and_then(|frame| {
                let local_start = frame.source_start_byte.checked_sub(clause_start)?;
                let tail = prefix.get(local_start..)?;
                let token = tail.split_whitespace().next()?;
                Some((
                    local_start,
                    token.to_string(),
                    frame.canonical_predicate.clone(),
                    builder.frame_events.get(&frame.frame_id).cloned(),
                ))
            })
            .or_else(|| {
                ["있는", "없는"]
                    .into_iter()
                    .filter_map(|marker| prefix.rfind(marker).map(|position| (position, marker)))
                    .max_by_key(|(position, _)| *position)
                    .map(|(position, marker)| {
                        (
                            position,
                            marker.to_string(),
                            if marker == "있는" { "HAS" } else { "LACKS" }.to_string(),
                            None,
                        )
                    })
            });
        let Some((predicate_start, predicate_surface, normalized_predicate, event_node_id)) =
            predicate
        else {
            continue;
        };
        let before_predicate = &prefix[..predicate_start];
        let dependent = builder
            .nodes
            .iter()
            .filter(|node| {
                node.kind == SemanticNodeKindIR::Entity
                    && node.node_id != head_node_id
                    && node.source_clause_id == outer.clause_id
                    && !matches!(node.normalized_label.as_str(), "있" | "없")
            })
            .filter_map(|node| {
                before_predicate
                    .rfind(node.surface.trim())
                    .or_else(|| before_predicate.rfind(&node.normalized_label))
                    .map(|position| (position, node.node_id.clone()))
            })
            .max_by_key(|(position, _)| *position);
        let Some((dependent_start, dependent_node_id)) = dependent else {
            continue;
        };
        let evidence_surface = prefix[dependent_start..].trim().to_string();
        if evidence_surface.is_empty() {
            continue;
        }
        if let Some(head_surface) = prefix.get(predicate_start + predicate_surface.len()..) {
            if let Some(head_node) = builder
                .nodes
                .iter_mut()
                .find(|node| node.node_id == head_node_id)
            {
                let normalized_head = normalize_korean_relative_head(head_surface);
                if !normalized_head.is_empty() {
                    head_node.normalized_label = normalized_head.clone();
                    head_node.concept_id_hint = concept_hint(&normalized_head).map(str::to_string);
                }
            }
        }
        builder
            .relative_clause_attachments
            .push(RelativeClauseAttachmentIR {
                attachment_id: String::new(),
                head_node_id,
                predicate_surface,
                normalized_predicate,
                dependent_node_ids: vec![dependent_node_id],
                embedded_event_node_id: event_node_id,
                negated: prefix.contains("없는") || prefix.contains("않"),
                evidence_surface,
                confidence_millis: 940,
            });
    }
}

fn primary_argument_id(builder: &GraphBuilder, frame_id: &str) -> Option<String> {
    let event_node_id = builder.frame_events.get(frame_id)?;
    for preferred in [
        SemanticRoleKindIR::Theme,
        SemanticRoleKindIR::Patient,
        SemanticRoleKindIR::Result,
        SemanticRoleKindIR::Destination,
        SemanticRoleKindIR::Topic,
    ] {
        if let Some(edge) = builder
            .role_edges
            .iter()
            .find(|edge| edge.event_node_id == *event_node_id && edge.role == preferred)
        {
            return Some(edge.argument_node_id.clone());
        }
    }
    None
}

fn infer_event_relations(text: &str, ordered: &[&PredicateFrameIR], builder: &mut GraphBuilder) {
    for pair in ordered.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let left_event = builder.frame_events.get(&left.frame_id).cloned();
        let right_event = builder.frame_events.get(&right.frame_id).cloned();
        let (Some(left_event), Some(right_event)) = (left_event, right_event) else {
            continue;
        };
        let between_start = left.source_start_byte + left.predicate_surface.len();
        let between = safe_slice(text, between_start, right.source_start_byte).to_lowercase();
        let (source, target, relation, evidence, confidence) = if contains_any(
            &between,
            &["때문에", "해서", "하므로", "므로", "so ", "therefore"],
        ) {
            (
                left_event.clone(),
                right_event.clone(),
                EventRelationKindIR::Cause,
                between.trim().to_string(),
                880,
            )
        } else if contains_any(&between, &["because", "since "]) {
            (
                right_event.clone(),
                left_event.clone(),
                EventRelationKindIR::Cause,
                between.trim().to_string(),
                820,
            )
        } else if contains_any(&between, &["면", "if ", "unless"]) {
            (
                left_event.clone(),
                right_event.clone(),
                EventRelationKindIR::Condition,
                between.trim().to_string(),
                850,
            )
        } else if contains_any(&between, &["위해", "하도록", "so that", "in order to"]) {
            (
                left_event.clone(),
                right_event.clone(),
                EventRelationKindIR::Purpose,
                between.trim().to_string(),
                850,
            )
        } else if contains_any(
            &between,
            &["뒤", "후", "다음", "then", "after that", "before"],
        ) {
            (
                left_event.clone(),
                right_event.clone(),
                EventRelationKindIR::TemporalBefore,
                between.trim().to_string(),
                930,
            )
        } else if contains_any(&between, &["하지만", "반면", "but", "however"]) {
            (
                left_event.clone(),
                right_event.clone(),
                EventRelationKindIR::Contrast,
                between.trim().to_string(),
                900,
            )
        } else if contains_any(&between, &["고", "그리고", "and", ","]) {
            (
                left_event,
                right_event,
                EventRelationKindIR::Coordination,
                between.trim().to_string(),
                780,
            )
        } else {
            continue;
        };
        builder.event_relations.push(EventRelationEdgeIR {
            source_event_node_id: source,
            target_event_node_id: target,
            relation,
            evidence_surface: evidence,
            confidence_millis: confidence,
        });
    }
}

fn korean_particle_role(particle: &str, frame: &PredicateFrameIR) -> SemanticRoleKindIR {
    match particle {
        "이" | "가" => SemanticRoleKindIR::Agent,
        "은" | "는" => SemanticRoleKindIR::Topic,
        "을" | "를" => SemanticRoleKindIR::Theme,
        "에게" | "한테" | "께" => SemanticRoleKindIR::Recipient,
        "에게서" | "한테서" | "으로부터" | "로부터" => SemanticRoleKindIR::Source,
        "와" | "과" | "하고" => {
            if predicate_matches(frame, &["비교", "compare"]) {
                SemanticRoleKindIR::ComparisonPeer
            } else {
                SemanticRoleKindIR::CoTheme
            }
        }
        "에서" => {
            if predicate_matches(frame, &["읽", "열", "가져", "불러", "move", "read", "open"])
            {
                SemanticRoleKindIR::Source
            } else {
                SemanticRoleKindIR::Location
            }
        }
        "에" | "까지" => {
            if predicate_matches(
                frame,
                &[
                    "저장", "배포", "옮", "보내", "save", "deploy", "move", "send",
                ],
            ) {
                SemanticRoleKindIR::Destination
            } else {
                SemanticRoleKindIR::Location
            }
        }
        "으로" | "로" => {
            if predicate_matches(frame, &["변환", "convert", "transform"]) {
                SemanticRoleKindIR::Result
            } else if predicate_matches(frame, &["옮", "배포", "move", "deploy"]) {
                SemanticRoleKindIR::Destination
            } else {
                SemanticRoleKindIR::Instrument
            }
        }
        _ => SemanticRoleKindIR::Theme,
    }
}

fn strip_korean_particle(token: &str) -> Option<(&str, &str)> {
    const PARTICLES: [&str; 23] = [
        "으로부터",
        "에게서",
        "한테서",
        "로부터",
        "에게",
        "한테",
        "에서",
        "까지",
        "으로",
        "하고",
        "께",
        "을",
        "를",
        "이",
        "가",
        "은",
        "는",
        "에",
        "로",
        "와",
        "과",
        "도",
        "만",
    ];
    PARTICLES.iter().find_map(|particle| {
        if *particle == "만" && token.ends_with("지만") {
            return None;
        }
        token
            .strip_suffix(particle)
            .filter(|base| !base.is_empty())
            .map(|base| (base, *particle))
    })
}

fn korean_phrase(words: &[WordSpan<'_>], index: usize, base: &str) -> String {
    let mut parts = vec![base.to_string()];
    for prior in words[..index].iter().rev().take(3) {
        let token = prior.text;
        if strip_korean_particle(token).is_some()
            || is_korean_boundary_word(token)
            || token.ends_with("지만")
            || token.ends_with(['고', '면'])
        {
            break;
        }
        parts.push(token.to_string());
    }
    parts.reverse();
    parts.join(" ")
}

fn is_korean_boundary_word(word: &str) -> bool {
    matches!(
        word,
        "그리고" | "하지만" | "그런데" | "뒤" | "후" | "다음" | "먼저" | "나중에"
    )
}

fn english_preposition(word: &str) -> Option<&str> {
    match word.to_lowercase().as_str() {
        "to" => Some("to"),
        "from" => Some("from"),
        "with" => Some("with"),
        "using" => Some("using"),
        "in" => Some("in"),
        "on" => Some("on"),
        "at" => Some("at"),
        "for" => Some("for"),
        "as" => Some("as"),
        "by" => Some("by"),
        "into" => Some("into"),
        "onto" => Some("onto"),
        _ => None,
    }
}

fn english_preposition_at<'a>(words: &[&str], index: usize, word: &'a str) -> Option<&'a str> {
    if word.eq_ignore_ascii_case("at")
        && words
            .get(index + 1)
            .is_some_and(|next| matches!(next.to_lowercase().as_str(), "least" | "most"))
    {
        None
    } else {
        english_preposition(word)
    }
}

fn english_preposition_role(preposition: &str, frame: &PredicateFrameIR) -> SemanticRoleKindIR {
    match preposition {
        "from" => SemanticRoleKindIR::Source,
        "with" | "using" => {
            if predicate_matches(frame, &["compare", "contrast"]) {
                SemanticRoleKindIR::ComparisonPeer
            } else {
                SemanticRoleKindIR::Instrument
            }
        }
        "in" | "on" | "at" => SemanticRoleKindIR::Location,
        "by" => SemanticRoleKindIR::Agent,
        "as" => SemanticRoleKindIR::Result,
        "into" | "onto" => SemanticRoleKindIR::Destination,
        "to" => {
            if predicate_matches(frame, &["tell", "send", "report", "communicate"]) {
                SemanticRoleKindIR::Recipient
            } else {
                SemanticRoleKindIR::Destination
            }
        }
        "for" => SemanticRoleKindIR::Recipient,
        _ => SemanticRoleKindIR::Theme,
    }
}

fn clean_english_subject(words: &[&str]) -> String {
    let filtered = words
        .iter()
        .copied()
        .filter(|word| {
            !matches!(
                word.to_lowercase().as_str(),
                "why" | "how" | "when" | "where" | "did" | "does" | "do" | "please"
            )
        })
        .collect::<Vec<_>>();
    clean_english_phrase(&filtered)
}

fn clean_english_phrase(words: &[&str]) -> String {
    let mut cleaned = words
        .iter()
        .copied()
        .take_while(|word| {
            !matches!(
                word.to_lowercase().as_str(),
                "then" | "but" | "because" | "if" | "unless"
            )
        })
        .filter(|word| {
            !matches!(
                word.to_lowercase().as_str(),
                "please" | "should" | "must" | "can" | "could" | "would" | "may" | "might"
            )
        })
        .collect::<Vec<_>>();
    while cleaned.first().is_some_and(|word| {
        matches!(
            word.to_lowercase().as_str(),
            "and" | "or" | "then" | "do" | "not" | "never"
        )
    }) {
        cleaned.remove(0);
    }
    while cleaned
        .last()
        .is_some_and(|word| matches!(word.to_lowercase().as_str(), "and" | "then" | "too"))
    {
        cleaned.pop();
    }
    cleaned.join(" ")
}

fn attach_clause_quantifier_fallback(
    builder: &mut GraphBuilder,
    event_node_id: &str,
    clause: &str,
) {
    let Some((quantifier, cardinality, negated, evidence)) = detect_quantifier(clause) else {
        return;
    };
    let Some(target) = builder
        .role_edges
        .iter()
        .find(|edge| {
            edge.event_node_id == event_node_id
                && matches!(
                    edge.role,
                    SemanticRoleKindIR::Theme | SemanticRoleKindIR::Patient
                )
        })
        .map(|edge| edge.argument_node_id.clone())
    else {
        return;
    };
    if let Some(scope) = builder
        .quantifier_scopes
        .iter_mut()
        .find(|scope| scope.target_node_id == target)
    {
        if negated {
            scope.quantifier = quantifier;
            scope.cardinality = cardinality;
            scope.negated = true;
            scope.evidence_surface = evidence;
            scope.confidence_millis = 900;
        }
        return;
    }
    builder.quantifier_scopes.push(QuantifierScopeIR {
        target_node_id: target,
        quantifier,
        cardinality,
        negated,
        evidence_surface: evidence,
        confidence_millis: 820,
    });
}

fn detect_quantifier(text: &str) -> Option<(QuantifierKindIR, Option<u64>, bool, String)> {
    let normalized = text.to_lowercase();
    if normalized.contains("아무") && contains_any(&normalized, &["않", "말", "지 마", "못", "없"])
    {
        return Some((
            QuantifierKindIR::None,
            None,
            true,
            "아무 + negation".to_string(),
        ));
    }
    for (markers, kind, negated) in [
        (
            &["최소", "적어도", "at least"][..],
            QuantifierKindIR::AtLeast,
            false,
        ),
        (
            &["최대", "많아도", "at most"][..],
            QuantifierKindIR::AtMost,
            false,
        ),
        (&["정확히", "exactly"][..], QuantifierKindIR::Exactly, false),
        (
            &["모든", "전부", "all ", "every "][..],
            QuantifierKindIR::All,
            false,
        ),
        (&["각", "각각", "each "][..], QuantifierKindIR::Each, false),
        (
            &["일부", "몇몇", "some "][..],
            QuantifierKindIR::Some,
            false,
        ),
        (&["아무", "any "][..], QuantifierKindIR::Any, false),
        (&["하나도", "no ", "none"][..], QuantifierKindIR::None, true),
    ] {
        if let Some(marker) = markers.iter().find(|marker| normalized.contains(**marker)) {
            let cardinality = if matches!(
                kind,
                QuantifierKindIR::Exactly | QuantifierKindIR::AtLeast | QuantifierKindIR::AtMost
            ) {
                first_unsigned_integer(&normalized)
            } else {
                None
            };
            return Some((kind, cardinality, negated, (*marker).trim().to_string()));
        }
    }
    None
}

fn first_unsigned_integer(text: &str) -> Option<u64> {
    text.split(|character: char| !character.is_ascii_digit())
        .find(|part| !part.is_empty())
        .and_then(|part| part.parse().ok())
}

fn predicate_matches(frame: &PredicateFrameIR, needles: &[&str]) -> bool {
    let surface = frame.predicate_surface.to_lowercase();
    let canonical = frame.canonical_predicate.to_lowercase();
    needles
        .iter()
        .any(|needle| surface.contains(needle) || canonical.contains(needle))
}

fn normalize_argument(surface: &str) -> String {
    surface
        .trim()
        .trim_matches(|character: char| {
            character.is_ascii_punctuation()
                || matches!(character, '‘' | '’' | '“' | '”' | '「' | '」' | '『' | '』')
        })
        .split_whitespace()
        .filter(|word| {
            let lower = word.to_lowercase();
            !matches!(
                lower.as_str(),
                "the"
                    | "a"
                    | "an"
                    | "please"
                    | "all"
                    | "every"
                    | "each"
                    | "some"
                    | "any"
                    | "no"
                    | "exactly"
                    | "least"
                    | "most"
                    | "at"
                    | "모든"
                    | "전부"
                    | "각"
                    | "각각"
                    | "일부"
                    | "몇몇"
                    | "아무"
                    | "하나도"
                    | "정확히"
                    | "최소"
                    | "최대"
                    | "적어도"
                    | "많아도"
            ) && !lower.chars().all(|character| character.is_ascii_digit())
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn normalize_korean_relative_head(surface: &str) -> String {
    let mut words = surface
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let Some(last) = words.last_mut() else {
        return String::new();
    };
    if let Some((base, _)) = strip_korean_particle(last) {
        *last = base.to_string();
    }
    normalize_argument(&words.join(" "))
}

fn is_non_argument(argument: &str) -> bool {
    matches!(
        argument,
        "" | "and" | "then" | "please" | "고" | "뒤" | "후" | "다음" | "그리고"
    )
}

fn concept_hint(argument: &str) -> Option<&'static str> {
    let normalized = argument.to_lowercase();
    for (surface, concept) in [
        ("repository", "C_OBJECT_REPOSITORY"),
        ("source code", "C_OBJECT_SOURCE_CODE"),
        ("저장소", "C_OBJECT_REPOSITORY"),
        ("보고서", "C_OBJECT_REPORT"),
        ("프로젝트", "C_OBJECT_PROJECT"),
        ("폴더", "C_OBJECT_FOLDER"),
        ("파일", "C_OBJECT_FILE"),
        ("문서", "C_OBJECT_DOCUMENT"),
        ("코드", "C_OBJECT_SOURCE_CODE"),
        ("오류", "C_OBJECT_DEFECT"),
        ("계획", "C_OBJECT_PLAN"),
        ("report", "C_OBJECT_REPORT"),
        ("project", "C_OBJECT_PROJECT"),
        ("folder", "C_OBJECT_FOLDER"),
        ("file", "C_OBJECT_FILE"),
        ("document", "C_OBJECT_DOCUMENT"),
        ("code", "C_OBJECT_SOURCE_CODE"),
        ("error", "C_OBJECT_DEFECT"),
        ("plan", "C_OBJECT_PLAN"),
    ] {
        if normalized.contains(surface) {
            return Some(concept);
        }
    }
    None
}

fn word_spans(text: &str) -> Vec<WordSpan<'_>> {
    let mut spans = Vec::new();
    let mut start = None;
    for (index, character) in text.char_indices() {
        if character.is_whitespace() || is_word_delimiter(character) {
            if let Some(word_start) = start.take() {
                spans.push(WordSpan {
                    text: &text[word_start..index],
                    start_byte: word_start,
                    end_byte: index,
                });
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(word_start) = start {
        spans.push(WordSpan {
            text: &text[word_start..],
            start_byte: word_start,
            end_byte: text.len(),
        });
    }
    spans
}

fn is_word_delimiter(character: char) -> bool {
    character.is_ascii_punctuation()
        || matches!(
            character,
            '‘' | '’' | '“' | '”' | '「' | '」' | '『' | '』' | '…'
        )
}

fn clause_bounds(text: &str, position: usize) -> (usize, usize) {
    let start = text[..position]
        .char_indices()
        .rev()
        .find(|(_, character)| is_clause_boundary(*character))
        .map_or(0, |(index, character)| index + character.len_utf8());
    let end = text[position..]
        .char_indices()
        .find(|(_, character)| is_clause_boundary(*character))
        .map_or(text.len(), |(offset, _)| position + offset);
    (start, end)
}

fn enclosing_quote_bounds(text: &str, position: usize) -> Option<(usize, usize)> {
    quote_ranges(text)
        .into_iter()
        .find(|(start, end)| position >= *start && position < *end)
        .map(|(start, end)| {
            let content_start = text[start..]
                .chars()
                .next()
                .map_or(start, |character| start + character.len_utf8());
            let closing_start = text[..end]
                .char_indices()
                .next_back()
                .map_or(end, |(index, _)| index);
            (content_start, closing_start)
        })
}

fn remove_quoted_content(text: &str) -> String {
    let ranges = quote_ranges(text);
    if ranges.is_empty() {
        return text.to_string();
    }
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end) in ranges {
        if start >= cursor && end <= text.len() {
            output.push_str(&text[cursor..start]);
            output.push(' ');
            cursor = end;
        }
    }
    output.push_str(&text[cursor..]);
    output
}

fn quote_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut stack = Vec::new();
    for (position, character) in text.char_indices() {
        match character {
            '‘' | '“' | '「' | '『' => stack.push((character, position)),
            '’' => close_quote_range(text, &mut stack, &mut ranges, position, '‘'),
            '”' => close_quote_range(text, &mut stack, &mut ranges, position, '“'),
            '」' => close_quote_range(text, &mut stack, &mut ranges, position, '「'),
            '』' => close_quote_range(text, &mut stack, &mut ranges, position, '『'),
            '"' | '\'' => {
                if let Some(stack_position) =
                    stack.iter().rposition(|(opening, _)| *opening == character)
                {
                    let (_, start) = stack.remove(stack_position);
                    ranges.push((start, position + character.len_utf8()));
                } else {
                    stack.push((character, position));
                }
            }
            _ => {}
        }
    }
    ranges.sort_unstable();
    ranges
}

fn close_quote_range(
    text: &str,
    stack: &mut Vec<(char, usize)>,
    ranges: &mut Vec<(usize, usize)>,
    position: usize,
    expected: char,
) {
    if let Some(stack_position) = stack.iter().rposition(|(opening, _)| *opening == expected) {
        let (_, start) = stack.remove(stack_position);
        let end = text[position..]
            .chars()
            .next()
            .map_or(position, |character| position + character.len_utf8());
        ranges.push((start, end));
    }
}

fn is_clause_boundary(character: char) -> bool {
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

fn contains_hangul(text: &str) -> bool {
    text.chars()
        .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character))
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositional_semantics::CompositionalSemanticAnalyzer;

    fn graph(text: &str) -> SemanticRoleGraphIR {
        CompositionalSemanticAnalyzer
            .analyze(text)
            .semantic_role_graph
    }

    fn has_role(graph: &SemanticRoleGraphIR, role: SemanticRoleKindIR, surface: &str) -> bool {
        graph.role_edges.iter().any(|edge| {
            edge.role == role
                && graph.nodes.iter().any(|node| {
                    node.node_id == edge.argument_node_id && node.surface.contains(surface)
                })
        })
    }

    #[test]
    fn korean_particles_form_agent_theme_source_and_destination_roles() {
        let graph = graph("사용자가 서버에서 모든 파일을 읽고 저장소에 저장해");
        assert!(graph.validate());
        assert!(has_role(&graph, SemanticRoleKindIR::Agent, "사용자"));
        assert!(has_role(&graph, SemanticRoleKindIR::Source, "서버"));
        assert!(has_role(&graph, SemanticRoleKindIR::Theme, "모든 파일"));
        assert!(has_role(&graph, SemanticRoleKindIR::Destination, "저장소"));
        assert!(graph
            .quantifier_scopes
            .iter()
            .any(|scope| scope.quantifier == QuantifierKindIR::All));
    }

    #[test]
    fn english_prepositions_and_passive_voice_bind_roles() {
        let graph = graph("the report was reviewed by Alice with a parser");
        assert!(graph.validate());
        assert!(has_role(&graph, SemanticRoleKindIR::Patient, "report"));
        assert!(has_role(&graph, SemanticRoleKindIR::Agent, "alice"));
        assert!(has_role(&graph, SemanticRoleKindIR::Instrument, "parser"));
    }

    #[test]
    fn sequence_and_prior_result_are_explicit_event_structure() {
        let graph = graph("파일을 읽고 변환한 뒤 저장해");
        assert!(graph.validate());
        assert!(graph
            .event_relations
            .iter()
            .any(|edge| { edge.relation == EventRelationKindIR::TemporalBefore }));
        assert!(graph
            .role_edges
            .iter()
            .any(|edge| edge.role == SemanticRoleKindIR::PriorResult));
    }

    #[test]
    fn korean_coordinate_gap_has_a_typed_forward_argument_binding() {
        let graph = graph("캐시를 확인하고 수리해");
        assert_eq!(graph.shared_argument_bindings.len(), 1);
        assert_eq!(
            graph.shared_argument_bindings[0].direction,
            SharedArgumentDirectionIR::Forward
        );
    }

    #[test]
    fn english_three_predicate_gap_has_two_backward_bindings() {
        let graph = graph("Read, transform, and save the file.");
        assert_eq!(graph.shared_argument_bindings.len(), 2);
        assert!(graph
            .shared_argument_bindings
            .iter()
            .all(|binding| binding.direction == SharedArgumentDirectionIR::Backward));
        let argument_ids = ["FRAME-01", "FRAME-02", "FRAME-03"]
            .into_iter()
            .map(|frame| {
                graph
                    .primary_argument_for_frame(frame)
                    .expect("shared argument")
                    .node_id
                    .as_str()
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(argument_ids.len(), 1);
    }

    #[test]
    fn explicit_distinct_arguments_are_never_replaced_by_sharing() {
        let graph = graph("Inspect the cache and repair the index.");
        assert!(graph.shared_argument_bindings.is_empty());
        assert_eq!(
            graph
                .primary_argument_for_frame("FRAME-01")
                .expect("cache")
                .normalized_label,
            "cache"
        );
        assert_eq!(
            graph
                .primary_argument_for_frame("FRAME-02")
                .expect("index")
                .normalized_label,
            "index"
        );
    }

    #[test]
    fn condition_relation_does_not_license_argument_sharing() {
        let graph = graph("If you inspect the cache, repair the index.");
        assert!(graph.shared_argument_bindings.is_empty());
        assert!(graph
            .event_relations
            .iter()
            .any(|edge| edge.relation == EventRelationKindIR::Condition));
    }

    #[test]
    fn temporal_prior_result_is_not_overwritten_by_argument_sharing() {
        let graph = graph("파일을 읽고 각 줄을 변환한 뒤 저장해");
        assert!(graph.shared_argument_bindings.is_empty());
        assert!(graph
            .arguments_for_frame("FRAME-03")
            .iter()
            .any(|(role, _)| { *role == SemanticRoleKindIR::PriorResult }));
    }

    #[test]
    fn focus_particle_is_an_explicit_argument_not_a_shareable_gap() {
        let graph = graph("오류를 고치지 말고 원인만 설명해");
        assert!(graph.shared_argument_bindings.is_empty());
        assert_eq!(
            graph
                .primary_argument_for_frame("FRAME-02")
                .expect("focused reason")
                .normalized_label,
            "원인"
        );
    }

    #[test]
    fn concessive_ending_is_not_split_as_the_focus_particle() {
        let graph = graph("민수는 파일을 삭제하라고 말했지만 이제 로그를 확인해");
        assert_eq!(
            graph
                .primary_argument_for_frame("FRAME-02")
                .expect("outer log")
                .normalized_label,
            "이제 로그"
        );
        assert!(!graph.nodes.iter().any(|node| node.surface == "말했지"));
    }

    #[test]
    fn shared_binding_cannot_gain_semantic_or_execution_authority() {
        let graph = graph("Inspect and repair the cache.");
        assert!(graph.validate());

        let mut semantic_tamper = graph.clone();
        semantic_tamper.shared_argument_bindings[0].semantic_authority = true;
        assert!(!semantic_tamper.validate());

        let mut execution_tamper = graph;
        execution_tamper.shared_argument_bindings[0].external_execution_authorized = true;
        assert!(!execution_tamper.validate());
    }

    #[test]
    fn numeric_quantifier_scope_retains_cardinality() {
        let graph = graph("analyze at least 3 files");
        let scope = graph.quantifier_scopes.first().expect("quantifier scope");
        assert_eq!(scope.quantifier, QuantifierKindIR::AtLeast);
        assert_eq!(scope.cardinality, Some(3));
    }

    #[test]
    fn relative_clause_keeps_head_and_embedded_event_as_typed_structure() {
        let graph = graph("파서가 수리한 모든 파일을 분석해");
        assert!(graph.validate());
        let attachment = graph
            .relative_clause_attachments
            .first()
            .expect("relative attachment");
        let head = graph
            .nodes
            .iter()
            .find(|node| node.node_id == attachment.head_node_id)
            .expect("relative head");
        assert_eq!(head.normalized_label, "파일");
        assert_eq!(attachment.normalized_predicate, "REPAIR");
        assert!(attachment.embedded_event_node_id.is_some());
    }

    #[test]
    fn nested_quantifiers_bind_to_distinct_relative_nodes() {
        let graph = graph("inspect every file that contains exactly 1 error");
        assert!(graph.validate());
        assert_eq!(graph.relative_clause_attachments.len(), 1);
        let targets = graph
            .quantifier_scopes
            .iter()
            .map(|scope| {
                (
                    scope.target_node_id.as_str(),
                    scope.quantifier,
                    scope.cardinality,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(targets.len(), 2);
        assert_ne!(targets[0].0, targets[1].0);
        assert!(targets.iter().any(|(_, kind, cardinality)| {
            *kind == QuantifierKindIR::Exactly && *cardinality == Some(1)
        }));
    }

    #[test]
    fn english_direct_object_coordination_creates_distinct_typed_arguments() {
        let graph = graph("Inspect the cache, the index, and the manifest.");
        let arguments = graph
            .arguments_for_frame("FRAME-01")
            .into_iter()
            .filter(|(role, _)| *role != SemanticRoleKindIR::Agent)
            .map(|(role, node)| (role, node.normalized_label.as_str()))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            arguments,
            BTreeSet::from([
                (SemanticRoleKindIR::Theme, "cache"),
                (SemanticRoleKindIR::CoTheme, "index"),
                (SemanticRoleKindIR::CoTheme, "manifest"),
            ])
        );
    }

    #[test]
    fn coordinated_argument_quantifiers_remain_member_local() {
        let graph = graph("Inspect every cache and each index.");
        let quantifiers = graph
            .quantifier_scopes
            .iter()
            .filter_map(|scope| {
                graph
                    .nodes
                    .iter()
                    .find(|node| node.node_id == scope.target_node_id)
                    .map(|node| (node.normalized_label.as_str(), scope.quantifier))
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            quantifiers,
            BTreeSet::from([
                ("cache", QuantifierKindIR::All),
                ("index", QuantifierKindIR::Each),
            ])
        );
    }

    #[test]
    fn passive_and_prepositional_coordination_preserve_base_roles() {
        let passive = graph("The cache and the index were inspected.");
        assert!(has_role(&passive, SemanticRoleKindIR::Patient, "cache"));
        assert!(has_role(&passive, SemanticRoleKindIR::CoTheme, "index"));

        let comparison = graph("Compare the cache with the index and the manifest.");
        let peers = comparison
            .arguments_for_frame("FRAME-01")
            .into_iter()
            .filter(|(role, _)| *role == SemanticRoleKindIR::ComparisonPeer)
            .map(|(_, node)| node.normalized_label.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(peers, BTreeSet::from(["index", "manifest"]));
    }

    #[test]
    fn coordinated_predicates_share_the_entire_argument_set() {
        for text in [
            "Inspect and repair the cache and the index.",
            "캐시와 인덱스를 점검하고 수리해.",
        ] {
            let graph = graph(text);
            assert_eq!(graph.shared_argument_bindings.len(), 2, "{text}");
            let first = graph
                .arguments_for_frame("FRAME-01")
                .into_iter()
                .filter(|(role, _)| *role != SemanticRoleKindIR::Agent)
                .map(|(role, node)| (role, node.node_id.as_str()))
                .collect::<BTreeSet<_>>();
            let second = graph
                .arguments_for_frame("FRAME-02")
                .into_iter()
                .filter(|(role, _)| *role != SemanticRoleKindIR::Agent)
                .map(|(role, node)| (role, node.node_id.as_str()))
                .collect::<BTreeSet<_>>();
            assert_eq!(first, second, "{text}");
            assert!(graph.shared_argument_bindings.iter().all(|binding| {
                !binding.semantic_authority && !binding.external_execution_authorized
            }));
        }
    }

    #[test]
    fn common_lexical_compound_is_not_silently_split() {
        let graph = graph("Inspect the research and development plan.");
        let arguments = graph
            .arguments_for_frame("FRAME-01")
            .into_iter()
            .filter(|(role, _)| *role != SemanticRoleKindIR::Agent)
            .collect::<Vec<_>>();
        assert_eq!(arguments.len(), 1);
        assert_eq!(
            arguments[0].1.normalized_label,
            "research and development plan"
        );
    }

    #[test]
    fn coordinated_condition_starts_after_the_direct_argument() {
        let graph =
            graph("Inspect the cache and if the cache is stale or damaged, repair the cache.");
        assert_eq!(
            graph
                .primary_argument_for_frame("FRAME-01")
                .expect("inspection theme")
                .normalized_label,
            "cache"
        );
        assert_eq!(
            graph
                .primary_argument_for_frame("FRAME-02")
                .expect("repair theme")
                .normalized_label,
            "cache"
        );
    }
}
