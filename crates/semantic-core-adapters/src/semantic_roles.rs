//! Typed semantic-role and proposition graph for the language adapter.
//!
//! Nodes in this graph are discourse-local bindings, not promoted semantic
//! concepts. Unknown noun phrases remain surface-grounded entity nodes with no
//! invented concept ID. The graph records who did what to which object, role
//! particles/prepositions, quantifier scope, and relations between events.

use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};

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
pub struct SemanticRoleGraphIR {
    pub schema: String,
    pub nodes: Vec<SemanticNodeIR>,
    pub role_edges: Vec<SemanticRoleEdgeIR>,
    pub event_relations: Vec<EventRelationEdgeIR>,
    pub quantifier_scopes: Vec<QuantifierScopeIR>,
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
        })
    }
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
        infer_event_relations(text, &ordered, &mut builder);
        builder.finish(frames.len())
    }
}

#[derive(Debug, Clone)]
struct WordSpan<'a> {
    text: &'a str,
}

#[derive(Default)]
struct GraphBuilder {
    nodes: Vec<SemanticNodeIR>,
    role_edges: Vec<SemanticRoleEdgeIR>,
    event_relations: Vec<EventRelationEdgeIR>,
    quantifier_scopes: Vec<QuantifierScopeIR>,
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
        if let Some((quantifier, cardinality, negated, quantifier_evidence)) =
            detect_quantifier(surface)
        {
            self.quantifier_scopes.push(QuantifierScopeIR {
                target_node_id: node_id.clone(),
                quantifier,
                cardinality,
                negated,
                evidence_surface: quantifier_evidence,
                confidence_millis: 900,
            });
        }
        Some(node_id)
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
    let before_words = word_spans(before)
        .into_iter()
        .map(|word| word.text)
        .collect::<Vec<_>>();
    let after_words = word_spans(after)
        .into_iter()
        .map(|word| word.text)
        .collect::<Vec<_>>();
    let passive_aux = before_words.iter().rposition(|word| {
        matches!(
            word.to_lowercase().as_str(),
            "is" | "are" | "was" | "were" | "be" | "been" | "being"
        )
    });
    if let Some(auxiliary) = passive_aux {
        let theme = clean_english_phrase(&before_words[..auxiliary]);
        builder.add_argument(
            event_node_id,
            &frame.clause_id,
            &theme,
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
    let direct_end = after_words
        .iter()
        .enumerate()
        .position(|(index, word)| english_preposition_at(&after_words, index, word).is_some())
        .unwrap_or(after_words.len());
    let direct = clean_english_phrase(&after_words[..direct_end]);
    if !direct.is_empty() {
        builder.add_argument(
            event_node_id,
            &frame.clause_id,
            &direct,
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
        let phrase = clean_english_phrase(&after_words[start..end]);
        let role = english_preposition_role(preposition, frame);
        builder.add_argument(
            event_node_id,
            &frame.clause_id,
            &phrase,
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
            let agent = clean_english_phrase(&after_words[by_index + 1..]);
            builder.add_argument(
                event_node_id,
                &frame.clause_id,
                &agent,
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
    const PARTICLES: [&str; 22] = [
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
    ];
    PARTICLES.iter().find_map(|particle| {
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
        if negated || !scope.negated {
            scope.quantifier = quantifier;
            scope.cardinality = cardinality;
            scope.negated = negated;
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
                });
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(word_start) = start {
        spans.push(WordSpan {
            text: &text[word_start..],
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
    fn numeric_quantifier_scope_retains_cardinality() {
        let graph = graph("analyze at least 3 files");
        let scope = graph.quantifier_scopes.first().expect("quantifier scope");
        assert_eq!(scope.quantifier, QuantifierKindIR::AtLeast);
        assert_eq!(scope.cardinality, Some(3));
    }
}
