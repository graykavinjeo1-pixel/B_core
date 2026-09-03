//! Hash-bound grammatical scope graph over discourse-local events and entities.
//!
//! The graph records inspectable language structure. Its nodes are not promoted
//! semantic concepts, and neither a lexical operator nor a well-formed graph
//! grants semantic truth or external execution authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compositional_semantics::{FramePolarityIR, PredicateFrameIR};
use crate::semantic_roles::{
    QuantifierKindIR, SemanticNodeIR, SemanticNodeKindIR, SemanticRoleGraphIR,
};

pub const GRAMMATICAL_SCOPE_GRAPH_SCHEMA: &str = "B_CORE_GRAMMATICAL_SCOPE_GRAPH_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrammaticalScopeNodeKindIR {
    Event,
    Entity,
    Quantifier,
    Negation,
    Restriction,
    Conjunction,
    Disjunction,
    FocusOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrammaticalScopeEdgeKindIR {
    Argument,
    AppliesTo,
    Governs,
    Restricts,
    Member,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammaticalScopeNodeIR {
    pub node_id: String,
    pub kind: GrammaticalScopeNodeKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_label: Option<String>,
    pub evidence_surface: String,
    pub confidence_millis: u16,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammaticalScopeEdgeIR {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub kind: GrammaticalScopeEdgeKindIR,
    pub evidence_surface: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammaticalScopeGraphIR {
    pub schema: String,
    pub nodes: Vec<GrammaticalScopeNodeIR>,
    pub edges: Vec<GrammaticalScopeEdgeIR>,
    pub root_node_ids: Vec<String>,
    pub unresolved_ambiguities: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
    pub graph_sha256: String,
}

impl Default for GrammaticalScopeGraphIR {
    fn default() -> Self {
        let mut graph = Self {
            schema: GRAMMATICAL_SCOPE_GRAPH_SCHEMA.to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            root_node_ids: Vec::new(),
            unresolved_ambiguities: Vec::new(),
            semantic_authority: false,
            external_execution_authorized: false,
            graph_sha256: String::new(),
        };
        graph.graph_sha256 = grammatical_scope_graph_sha256(&graph);
        graph
    }
}

impl GrammaticalScopeGraphIR {
    pub fn validate(&self) -> bool {
        if self.schema != GRAMMATICAL_SCOPE_GRAPH_SCHEMA
            || self.semantic_authority
            || self.external_execution_authorized
            || self.graph_sha256.len() != 64
            || grammatical_scope_graph_sha256(self) != self.graph_sha256
        {
            return false;
        }
        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.node_id.as_str())
            .collect::<BTreeSet<_>>();
        let edge_ids = self
            .edges
            .iter()
            .map(|edge| edge.edge_id.as_str())
            .collect::<BTreeSet<_>>();
        if node_ids.len() != self.nodes.len()
            || edge_ids.len() != self.edges.len()
            || self.nodes.iter().any(|node| {
                node.node_id.trim().is_empty()
                    || node.evidence_surface.trim().is_empty()
                    || node.confidence_millis > 1_000
                    || node.semantic_authority
                    || node.external_execution_authorized
                    || matches!(
                        node.kind,
                        GrammaticalScopeNodeKindIR::Event | GrammaticalScopeNodeKindIR::Entity
                    ) && node.reference_id.as_deref().is_none_or(str::is_empty)
                    || !matches!(
                        node.kind,
                        GrammaticalScopeNodeKindIR::Event | GrammaticalScopeNodeKindIR::Entity
                    ) && node.operator_label.as_deref().is_none_or(str::is_empty)
            })
            || self.edges.iter().any(|edge| {
                edge.edge_id.trim().is_empty()
                    || edge.source_node_id == edge.target_node_id
                    || !node_ids.contains(edge.source_node_id.as_str())
                    || !node_ids.contains(edge.target_node_id.as_str())
                    || edge.evidence_surface.trim().is_empty()
                    || edge.confidence_millis > 1_000
            })
            || self
                .root_node_ids
                .iter()
                .any(|root| !node_ids.contains(root.as_str()))
        {
            return false;
        }
        let roots = self
            .root_node_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        roots.len() == self.root_node_ids.len()
            && acyclic(&node_ids, &self.edges)
            && self
                .unresolved_ambiguities
                .iter()
                .all(|ambiguity| !ambiguity.trim().is_empty())
    }
}

pub fn grammatical_scope_graph_sha256(graph: &GrammaticalScopeGraphIR) -> String {
    let bytes = serde_json::to_vec(&(
        &graph.schema,
        &graph.nodes,
        &graph.edges,
        &graph.root_node_ids,
        &graph.unresolved_ambiguities,
        graph.semantic_authority,
        graph.external_execution_authorized,
    ))
    .expect("grammatical scope graph serializes");
    format!("{:x}", Sha256::digest(bytes))
}

fn acyclic(node_ids: &BTreeSet<&str>, edges: &[GrammaticalScopeEdgeIR]) -> bool {
    let mut indegree = node_ids
        .iter()
        .map(|node| ((*node).to_string(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    for edge in edges {
        *indegree.entry(edge.target_node_id.clone()).or_default() += 1;
        outgoing
            .entry(edge.source_node_id.clone())
            .or_default()
            .push(edge.target_node_id.clone());
    }
    let mut ready = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(node, _)| node.clone())
        .collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(node) = ready.pop() {
        visited += 1;
        for target in outgoing.get(&node).into_iter().flatten() {
            let Some(degree) = indegree.get_mut(target) else {
                return false;
            };
            *degree = degree.saturating_sub(1);
            if *degree == 0 {
                ready.push(target.clone());
            }
        }
    }
    visited == node_ids.len()
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GrammaticalScopeAnalyzer;

impl GrammaticalScopeAnalyzer {
    pub fn analyze(
        &self,
        text: &str,
        frames: &[PredicateFrameIR],
        role_graph: &SemanticRoleGraphIR,
    ) -> GrammaticalScopeGraphIR {
        let mut builder = ScopeGraphBuilder::default();
        for frame in frames {
            builder.add_reference(
                format!("SCOPE-EVENT-{}", frame.frame_id),
                GrammaticalScopeNodeKindIR::Event,
                &frame.frame_id,
                &frame.predicate_surface,
            );
        }
        for node in role_graph
            .nodes
            .iter()
            .filter(|node| node.kind == SemanticNodeKindIR::Entity)
        {
            builder.add_reference(
                scope_entity_id(&node.node_id),
                GrammaticalScopeNodeKindIR::Entity,
                &node.node_id,
                &node.surface,
            );
        }
        for role in &role_graph.role_edges {
            let event_id = role_graph
                .nodes
                .iter()
                .find(|node| node.node_id == role.event_node_id)
                .and_then(|node| node.source_frame_id.as_deref())
                .map(|frame_id| format!("SCOPE-EVENT-{frame_id}"));
            let entity_id = role_graph
                .nodes
                .iter()
                .find(|node| {
                    node.node_id == role.argument_node_id && node.kind == SemanticNodeKindIR::Entity
                })
                .map(|node| scope_entity_id(&node.node_id));
            if let (Some(event_id), Some(entity_id)) = (event_id, entity_id) {
                builder.add_edge(
                    &event_id,
                    &entity_id,
                    GrammaticalScopeEdgeKindIR::Argument,
                    format!("{:?}:{}", role.role, role.evidence_surface),
                    role.confidence_millis,
                );
            }
        }

        let mut quantifier_nodes = BTreeMap::<String, Vec<(QuantifierKindIR, String)>>::new();
        let mut seen_quantifiers = BTreeSet::new();
        for scope in &role_graph.quantifier_scopes {
            if !seen_quantifiers.insert((
                scope.target_node_id.clone(),
                scope.quantifier,
                scope.cardinality,
                scope.negated,
            )) {
                continue;
            }
            let target_id = scope_entity_id(&scope.target_node_id);
            if !builder.contains(&target_id) {
                continue;
            }
            let label = quantifier_label(scope.quantifier, scope.cardinality);
            let quantifier_id = builder.add_operator(
                GrammaticalScopeNodeKindIR::Quantifier,
                &label,
                &scope.evidence_surface,
                scope.confidence_millis,
            );
            builder.add_edge(
                &quantifier_id,
                &target_id,
                GrammaticalScopeEdgeKindIR::AppliesTo,
                &scope.evidence_surface,
                scope.confidence_millis,
            );
            quantifier_nodes
                .entry(scope.target_node_id.clone())
                .or_default()
                .push((scope.quantifier, quantifier_id));
        }

        let mut negation_for_frame = BTreeMap::new();
        for frame in frames
            .iter()
            .filter(|frame| frame.polarity == FramePolarityIR::Negative)
        {
            let negation_id = builder.add_operator(
                GrammaticalScopeNodeKindIR::Negation,
                "NOT",
                &frame.predicate_surface,
                980,
            );
            let event_id = format!("SCOPE-EVENT-{}", frame.frame_id);
            builder.add_edge(
                &negation_id,
                &event_id,
                GrammaticalScopeEdgeKindIR::Governs,
                &frame.predicate_surface,
                980,
            );
            negation_for_frame.insert(frame.frame_id.clone(), negation_id);
        }

        let mut scope_target_ids = role_graph
            .quantifier_scopes
            .iter()
            .map(|scope| scope.target_node_id.clone())
            .collect::<BTreeSet<_>>();
        scope_target_ids.extend(
            frames
                .iter()
                .filter_map(|frame| role_graph.primary_argument_for_frame(&frame.frame_id))
                .map(|node| node.node_id.clone()),
        );
        scope_target_ids.extend(
            role_graph
                .relative_clause_attachments
                .iter()
                .map(|attachment| attachment.head_node_id.clone()),
        );
        for entity in role_graph.nodes.iter().filter(|node| {
            node.kind == SemanticNodeKindIR::Entity && scope_target_ids.contains(&node.node_id)
        }) {
            let target_id = scope_entity_id(&entity.node_id);
            if let Some(expression) = restriction_expression(text, entity) {
                let root = builder.materialize_restriction(&expression);
                builder.add_edge(
                    &root,
                    &target_id,
                    GrammaticalScopeEdgeKindIR::Restricts,
                    expression.evidence(),
                    900,
                );
            }
            if focus_only_applies(text, entity) {
                let focus_id = builder.add_operator(
                    GrammaticalScopeNodeKindIR::FocusOnly,
                    "ONLY",
                    focus_evidence(text, entity),
                    930,
                );
                builder.add_edge(
                    &focus_id,
                    &target_id,
                    GrammaticalScopeEdgeKindIR::AppliesTo,
                    focus_evidence(text, entity),
                    930,
                );
            }
        }

        for frame in frames
            .iter()
            .filter(|frame| frame.polarity == FramePolarityIR::Negative)
        {
            let Some(negation_id) = negation_for_frame.get(&frame.frame_id) else {
                continue;
            };
            let Some(event_node) = role_graph.event_node_for_frame(&frame.frame_id) else {
                continue;
            };
            for role in role_graph
                .role_edges
                .iter()
                .filter(|role| role.event_node_id == event_node.node_id)
            {
                for (quantifier, quantifier_id) in quantifier_nodes
                    .get(&role.argument_node_id)
                    .into_iter()
                    .flatten()
                {
                    if !matches!(quantifier, QuantifierKindIR::None) {
                        builder.unresolved_ambiguities.insert(format!(
                            "NEGATION_QUANTIFIER_SCOPE:{}:{}:{}",
                            frame.frame_id, negation_id, quantifier_id
                        ));
                    }
                }
            }
        }
        builder.finish()
    }
}

fn scope_entity_id(node_id: &str) -> String {
    format!("SCOPE-ENTITY-{node_id}")
}

fn quantifier_label(kind: QuantifierKindIR, cardinality: Option<u64>) -> String {
    let base = format!("{kind:?}").to_uppercase();
    cardinality.map_or(base.clone(), |value| format!("{base}:{value}"))
}

#[derive(Debug, Clone)]
enum RestrictionExpression {
    Atom { surface: String, negated: bool },
    And(Vec<Self>),
    Or(Vec<Self>),
}

impl RestrictionExpression {
    fn evidence(&self) -> &str {
        match self {
            Self::Atom { surface, .. } => surface,
            Self::And(children) | Self::Or(children) => {
                children.first().map_or("restriction", Self::evidence)
            }
        }
    }
}

fn restriction_expression(text: &str, entity: &SemanticNodeIR) -> Option<RestrictionExpression> {
    if text.is_ascii() {
        english_restriction(text, entity)
    } else {
        korean_restriction(text, entity)
    }
}

fn english_restriction(text: &str, entity: &SemanticNodeIR) -> Option<RestrictionExpression> {
    let lower = text.to_lowercase();
    let head = english_head(entity)?;
    for marker in [format!("{head} that "), format!("{head} which ")] {
        if let Some(start) = lower.find(&marker) {
            let body_start = start + marker.len();
            let body = lower[body_start..]
                .split(['.', '?', '!', ';'])
                .next()
                .unwrap_or_default()
                .trim();
            if !body.is_empty() {
                return parse_english_boolean(body);
            }
        }
    }
    let surface = entity.surface.to_lowercase();
    let words = surface
        .split_whitespace()
        .map(clean_ascii_word)
        .filter(|word| !word.is_empty())
        .filter(|word| {
            !matches!(
                word.as_str(),
                "the" | "a" | "an" | "all" | "every" | "each" | "some" | "any" | "no" | "only"
            )
        })
        .collect::<Vec<_>>();
    if words.len() < 2 {
        return None;
    }
    let modifiers = words[..words.len() - 1].join(" ");
    (!modifiers.is_empty()).then_some(RestrictionExpression::Atom {
        surface: modifiers,
        negated: false,
    })
}

fn english_head(entity: &SemanticNodeIR) -> Option<String> {
    entity
        .normalized_label
        .split_whitespace()
        .next_back()
        .map(clean_ascii_word)
        .filter(|head| !head.is_empty())
        .or_else(|| {
            entity
                .surface
                .split_whitespace()
                .next_back()
                .map(clean_ascii_word)
                .filter(|head| !head.is_empty())
        })
}

fn clean_ascii_word(word: &str) -> String {
    word.trim_matches(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .to_lowercase()
}

fn parse_english_boolean(body: &str) -> Option<RestrictionExpression> {
    let or_parts = split_nonempty(body, &[" or "]);
    if or_parts.len() > 1 {
        return Some(RestrictionExpression::Or(
            or_parts
                .into_iter()
                .filter_map(parse_english_boolean)
                .collect(),
        ));
    }
    let and_parts = split_nonempty(body, &[" and ", " but "]);
    if and_parts.len() > 1 {
        return Some(RestrictionExpression::And(
            and_parts
                .into_iter()
                .filter_map(parse_english_boolean)
                .collect(),
        ));
    }
    let mut atom = body.trim();
    let mut negated = false;
    for prefix in ["not ", "no ", "never "] {
        if let Some(rest) = atom.strip_prefix(prefix) {
            atom = rest.trim();
            negated = true;
            break;
        }
    }
    for prefix in ["is ", "are ", "was ", "were "] {
        if let Some(rest) = atom.strip_prefix(prefix) {
            atom = rest.trim();
            break;
        }
    }
    (!atom.is_empty()).then(|| RestrictionExpression::Atom {
        surface: atom.to_string(),
        negated,
    })
}

fn korean_restriction(text: &str, entity: &SemanticNodeIR) -> Option<RestrictionExpression> {
    let head = korean_head(entity)?;
    let head_start = text.rfind(&head)?;
    let boundary = text[..head_start]
        .rfind(['.', '?', '!', ';', ','])
        .map_or(0, |index| index + 1);
    let mut prefix = text[boundary..head_start].trim().to_string();
    for quantifier in ["모든", "전부", "각각", "각", "일부", "어떤", "하나도"] {
        prefix = prefix.replace(quantifier, " ");
    }
    let prefix = prefix.split_whitespace().collect::<Vec<_>>().join(" ");
    if prefix.is_empty() || korean_prefix_is_only_role_material(&prefix) {
        return None;
    }
    parse_korean_boolean(&prefix)
}

fn korean_head(entity: &SemanticNodeIR) -> Option<String> {
    entity
        .surface
        .split_whitespace()
        .next_back()
        .map(|word| {
            word.trim_matches(|character: char| {
                character.is_ascii_punctuation()
                    || matches!(character, '을' | '를' | '은' | '는' | '이' | '가' | '만')
            })
            .to_string()
        })
        .filter(|head| !head.is_empty())
}

fn korean_prefix_is_only_role_material(prefix: &str) -> bool {
    prefix.split_whitespace().all(|word| {
        word.ends_with('가')
            || word.ends_with('이')
            || word.ends_with("에서")
            || word.ends_with("에게")
    })
}

fn parse_korean_boolean(body: &str) -> Option<RestrictionExpression> {
    if let Some((left, right)) = split_korean_once(body, &["거나 ", "든지 "]) {
        return Some(RestrictionExpression::Or(vec![
            parse_korean_boolean(left)?,
            parse_korean_boolean(right)?,
        ]));
    }
    if let Some((left, right)) = split_korean_once(body, &["지만 ", "으며 ", "면서 ", "고 "])
    {
        return Some(RestrictionExpression::And(vec![
            parse_korean_boolean(left)?,
            parse_korean_boolean(right)?,
        ]));
    }
    let atom = body.trim();
    if atom.is_empty() {
        return None;
    }
    let negated = ["지 않", "않은", "없는", "못 ", "아닌"]
        .iter()
        .any(|marker| atom.contains(marker));
    Some(RestrictionExpression::Atom {
        surface: atom.to_string(),
        negated,
    })
}

fn split_nonempty<'a>(text: &'a str, markers: &[&str]) -> Vec<&'a str> {
    for marker in markers {
        let parts = text
            .split(marker)
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() > 1 {
            return parts;
        }
    }
    vec![text.trim()]
}

fn split_korean_once<'a>(text: &'a str, markers: &[&str]) -> Option<(&'a str, &'a str)> {
    markers.iter().find_map(|marker| {
        text.find(marker).and_then(|index| {
            let left = text[..index].trim();
            let right = text[index + marker.len()..].trim();
            (!left.is_empty() && !right.is_empty()).then_some((left, right))
        })
    })
}

fn focus_only_applies(text: &str, entity: &SemanticNodeIR) -> bool {
    if text.is_ascii() {
        let lower = text.to_lowercase();
        entity.surface.to_lowercase().starts_with("only ")
            || english_head(entity).is_some_and(|head| lower.contains(&format!("only {head}")))
    } else {
        !entity.surface.ends_with("지만")
            && korean_head(entity).is_some_and(|head| text.contains(&format!("{head}만")))
    }
}

fn focus_evidence<'a>(text: &'a str, entity: &'a SemanticNodeIR) -> &'a str {
    if text.is_ascii() {
        "only"
    } else if text.contains('만') {
        "만"
    } else {
        entity.surface.as_str()
    }
}

#[derive(Default)]
struct ScopeGraphBuilder {
    nodes: Vec<GrammaticalScopeNodeIR>,
    edges: Vec<GrammaticalScopeEdgeIR>,
    counters: BTreeMap<GrammaticalScopeNodeKindIR, usize>,
    unresolved_ambiguities: BTreeSet<String>,
}

impl ScopeGraphBuilder {
    fn contains(&self, node_id: &str) -> bool {
        self.nodes.iter().any(|node| node.node_id == node_id)
    }

    fn add_reference(
        &mut self,
        node_id: String,
        kind: GrammaticalScopeNodeKindIR,
        reference_id: &str,
        evidence: &str,
    ) {
        if self.contains(&node_id) || evidence.trim().is_empty() {
            return;
        }
        self.nodes.push(GrammaticalScopeNodeIR {
            node_id,
            kind,
            reference_id: Some(reference_id.to_string()),
            operator_label: None,
            evidence_surface: evidence.trim().to_string(),
            confidence_millis: 1_000,
            semantic_authority: false,
            external_execution_authorized: false,
        });
    }

    fn add_operator(
        &mut self,
        kind: GrammaticalScopeNodeKindIR,
        label: &str,
        evidence: &str,
        confidence_millis: u16,
    ) -> String {
        let counter = self.counters.entry(kind).or_default();
        *counter += 1;
        let node_id = format!("SCOPE-{}-{counter:03}", scope_kind_label(kind));
        self.nodes.push(GrammaticalScopeNodeIR {
            node_id: node_id.clone(),
            kind,
            reference_id: None,
            operator_label: Some(label.to_string()),
            evidence_surface: if evidence.trim().is_empty() {
                label.to_string()
            } else {
                evidence.trim().to_string()
            },
            confidence_millis,
            semantic_authority: false,
            external_execution_authorized: false,
        });
        node_id
    }

    fn add_edge(
        &mut self,
        source: &str,
        target: &str,
        kind: GrammaticalScopeEdgeKindIR,
        evidence: impl AsRef<str>,
        confidence_millis: u16,
    ) {
        if source == target
            || !self.contains(source)
            || !self.contains(target)
            || self.edges.iter().any(|edge| {
                edge.source_node_id == source && edge.target_node_id == target && edge.kind == kind
            })
        {
            return;
        }
        let evidence = evidence.as_ref().trim();
        self.edges.push(GrammaticalScopeEdgeIR {
            edge_id: String::new(),
            source_node_id: source.to_string(),
            target_node_id: target.to_string(),
            kind,
            evidence_surface: if evidence.is_empty() {
                format!("{kind:?}")
            } else {
                evidence.to_string()
            },
            confidence_millis,
        });
    }

    fn materialize_restriction(&mut self, expression: &RestrictionExpression) -> String {
        match expression {
            RestrictionExpression::Atom { surface, negated } => {
                let restriction_id = self.add_operator(
                    GrammaticalScopeNodeKindIR::Restriction,
                    &restriction_label(surface),
                    surface,
                    900,
                );
                if *negated {
                    let negation_id = self.add_operator(
                        GrammaticalScopeNodeKindIR::Negation,
                        "NOT",
                        surface,
                        900,
                    );
                    self.add_edge(
                        &negation_id,
                        &restriction_id,
                        GrammaticalScopeEdgeKindIR::Governs,
                        surface,
                        900,
                    );
                    negation_id
                } else {
                    restriction_id
                }
            }
            RestrictionExpression::And(children) | RestrictionExpression::Or(children) => {
                let (kind, label) = match expression {
                    RestrictionExpression::And(_) => {
                        (GrammaticalScopeNodeKindIR::Conjunction, "AND")
                    }
                    RestrictionExpression::Or(_) => (GrammaticalScopeNodeKindIR::Disjunction, "OR"),
                    RestrictionExpression::Atom { .. } => unreachable!(),
                };
                let root = self.add_operator(kind, label, expression.evidence(), 900);
                for child in children {
                    let child_id = self.materialize_restriction(child);
                    self.add_edge(
                        &root,
                        &child_id,
                        GrammaticalScopeEdgeKindIR::Member,
                        label,
                        900,
                    );
                }
                root
            }
        }
    }

    fn finish(mut self) -> GrammaticalScopeGraphIR {
        self.nodes
            .sort_by(|left, right| left.node_id.cmp(&right.node_id));
        self.edges.sort_by(|left, right| {
            left.source_node_id
                .cmp(&right.source_node_id)
                .then_with(|| left.target_node_id.cmp(&right.target_node_id))
                .then_with(|| left.kind.cmp(&right.kind))
        });
        for (index, edge) in self.edges.iter_mut().enumerate() {
            edge.edge_id = format!("SCOPE-EDGE-{:03}", index + 1);
        }
        let targets = self
            .edges
            .iter()
            .map(|edge| edge.target_node_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut root_node_ids = self
            .nodes
            .iter()
            .filter(|node| !targets.contains(node.node_id.as_str()))
            .map(|node| node.node_id.clone())
            .collect::<Vec<_>>();
        root_node_ids.sort();
        let mut graph = GrammaticalScopeGraphIR {
            schema: GRAMMATICAL_SCOPE_GRAPH_SCHEMA.to_string(),
            nodes: self.nodes,
            edges: self.edges,
            root_node_ids,
            unresolved_ambiguities: self.unresolved_ambiguities.into_iter().collect(),
            semantic_authority: false,
            external_execution_authorized: false,
            graph_sha256: String::new(),
        };
        graph.graph_sha256 = grammatical_scope_graph_sha256(&graph);
        debug_assert!(graph.validate());
        graph
    }
}

fn restriction_label(surface: &str) -> String {
    let cleaned = surface
        .trim()
        .trim_end_matches(['.', '?', '!', ',', ';'])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_");
    if cleaned.is_ascii() {
        cleaned.to_uppercase()
    } else {
        cleaned
    }
}

const fn scope_kind_label(kind: GrammaticalScopeNodeKindIR) -> &'static str {
    match kind {
        GrammaticalScopeNodeKindIR::Event => "EVENT",
        GrammaticalScopeNodeKindIR::Entity => "ENTITY",
        GrammaticalScopeNodeKindIR::Quantifier => "QUANTIFIER",
        GrammaticalScopeNodeKindIR::Negation => "NEGATION",
        GrammaticalScopeNodeKindIR::Restriction => "RESTRICTION",
        GrammaticalScopeNodeKindIR::Conjunction => "CONJUNCTION",
        GrammaticalScopeNodeKindIR::Disjunction => "DISJUNCTION",
        GrammaticalScopeNodeKindIR::FocusOnly => "FOCUS-ONLY",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompositionalSemanticAnalyzer;

    fn graph(text: &str) -> GrammaticalScopeGraphIR {
        CompositionalSemanticAnalyzer
            .analyze(text)
            .grammatical_scope_graph
    }

    #[test]
    fn recursive_restriction_is_hash_bound_and_non_authoritative() {
        let graph = graph("Repair every cache that is stale and not locked.");
        assert!(graph.validate());
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == GrammaticalScopeNodeKindIR::Conjunction));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == GrammaticalScopeNodeKindIR::Negation));
        assert!(!graph.semantic_authority);
        assert!(!graph.external_execution_authorized);
    }

    #[test]
    fn graph_tampering_fails_validation() {
        let graph = graph("Inspect each file that contains errors or lacks metadata.");
        let mut tampered = graph.clone();
        tampered.nodes[0].semantic_authority = true;
        tampered.graph_sha256 = grammatical_scope_graph_sha256(&tampered);
        assert!(!tampered.validate());
        let mut rewired = graph;
        rewired.edges[0].target_node_id = "MISSING".to_string();
        rewired.graph_sha256 = grammatical_scope_graph_sha256(&rewired);
        assert!(!rewired.validate());
    }

    #[test]
    fn korean_recursive_restriction_preserves_shared_primary_argument() {
        let analysis = CompositionalSemanticAnalyzer
            .analyze("오래됐지만 잠기지 않은 모든 캐시를 검사하고 수리해");
        let primary = analysis
            .frames
            .iter()
            .map(|frame| {
                analysis
                    .semantic_role_graph
                    .primary_argument_for_frame(&frame.frame_id)
                    .map(|node| node.node_id.clone())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            primary.iter().flatten().collect::<BTreeSet<_>>().len(),
            1,
            "primary={primary:?}; edges={:?}; bindings={:?}",
            analysis.semantic_role_graph.role_edges,
            analysis.semantic_role_graph.shared_argument_bindings
        );
        assert!(primary.iter().all(Option::is_some));
    }
}
