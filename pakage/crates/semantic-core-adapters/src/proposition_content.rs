//! Typed, source-bound content slots for dialogue propositions. Unlike the
//! action planner, this compiler can retain a causal connective even when its
//! predicates have no executable operator. All content remains attributed data.

use crate::compositional_semantics::CompositionalSemanticAnalyzer;
use crate::semantic_roles::SemanticRoleKindIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContentSlotIR {
    Cause,
    Agent,
    Theme,
    Definition,
    Summary,
    Manner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentBindingIR {
    pub slot: ContentSlotIR,
    pub value: String,
    pub predicate: Option<String>,
    pub grammar_evidence: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionContentIR {
    pub source_sha256: String,
    pub bindings: Vec<ContentBindingIR>,
}

impl PropositionContentIR {
    pub fn compile(source: &str) -> Self {
        let mut bindings = Vec::new();
        let parsed = CompositionalSemanticAnalyzer.analyze(source);
        let graph = &parsed.semantic_role_graph;
        for edge in &graph.role_edges {
            let slot = match edge.role {
                SemanticRoleKindIR::Agent => ContentSlotIR::Agent,
                SemanticRoleKindIR::Theme | SemanticRoleKindIR::Patient => ContentSlotIR::Theme,
                _ => continue,
            };
            let Some(node) = graph
                .nodes
                .iter()
                .find(|node| node.node_id == edge.argument_node_id)
            else {
                continue;
            };
            if node.kind == crate::semantic_roles::SemanticNodeKindIR::ImplicitAgent {
                continue;
            }
            bindings.push(ContentBindingIR {
                slot,
                value: node.surface.clone(),
                predicate: graph
                    .nodes
                    .iter()
                    .find(|node| node.node_id == edge.event_node_id)
                    .map(|node| node.normalized_label.clone()),
                grammar_evidence: format!("ROLE:{:?}:{}", edge.role, edge.evidence_surface),
            });
        }
        // ASCII folding preserves byte offsets and the original entity casing.
        let lower = source.to_ascii_lowercase();
        if let Some((effect, cause)) = lower.split_once(" because ") {
            if !effect.trim().is_empty() && !cause.trim().is_empty() {
                bindings.push(ContentBindingIR {
                    slot: ContentSlotIR::Cause,
                    value: source[effect.len() + " because ".len()..]
                        .trim()
                        .to_string(),
                    predicate: None,
                    grammar_evidence: "CAUSAL_COMPLEMENT:because".into(),
                });
            }
        } else {
            // -서 can mark either sequence or cause. Do not silently select
            // causality from that ending alone. Require an explicit connective.
            for connector in [" 때문에 "] {
                if let Some(position) = lower.find(connector) {
                    if position > 0 && position + connector.len() < lower.len() {
                        bindings.push(ContentBindingIR {
                            slot: ContentSlotIR::Cause,
                            value: source[..position + connector.len()].trim().to_string(),
                            predicate: None,
                            grammar_evidence: format!("CAUSAL_CONNECTIVE:{}", connector.trim()),
                        });
                        break;
                    }
                }
            }
        }
        if let Some((subject, definition)) = lower.split_once(" means ") {
            if !subject.trim().is_empty() && !definition.trim().is_empty() {
                bindings.push(ContentBindingIR {
                    slot: ContentSlotIR::Definition,
                    value: source[subject.len() + " means ".len()..].trim().to_string(),
                    predicate: None,
                    grammar_evidence: "DEFINITION_COMPLEMENT:means".into(),
                });
            }
        }
        bindings.truncate(16);
        Self {
            source_sha256: format!("{:x}", Sha256::digest(source.as_bytes())),
            bindings,
        }
    }

    pub fn validate_source(&self, source: &str) -> bool {
        *self == Self::compile(source)
    }
}

pub fn requested_content_slot(text: &str) -> Option<ContentSlotIR> {
    let lower = text.trim().to_lowercase();
    let words = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if words.iter().any(|word| {
        matches!(
            *word,
            "why" | "reason" | "cause" | "왜" | "이유" | "이유만" | "원인"
        )
    }) {
        return Some(ContentSlotIR::Cause);
    }
    if words
        .first()
        .is_some_and(|word| matches!(*word, "who" | "누가" | "누구"))
    {
        return Some(ContentSlotIR::Agent);
    }
    if words.iter().any(|word| matches!(*word, "뭘" | "무엇을")) || lower.starts_with("what did ")
    {
        return Some(ContentSlotIR::Theme);
    }
    if lower.starts_with("what is ")
        || lower.contains("what a ")
        || lower.contains("뭔지")
        || lower.contains("정의")
    {
        return Some(ContentSlotIR::Definition);
    }
    if words.iter().any(|word| matches!(*word, "how" | "어떻게")) {
        return Some(ContentSlotIR::Manner);
    }
    if words
        .iter()
        .any(|word| matches!(*word, "summarize" | "summary" | "요약해"))
    {
        return Some(ContentSlotIR::Summary);
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentProjectionIR {
    pub belief_id: String,
    pub source_actor: String,
    pub source_proposition: String,
    pub binding: ContentBindingIR,
}

impl ContentProjectionIR {
    pub fn validate(&self) -> bool {
        !self.belief_id.is_empty()
            && !self.source_actor.is_empty()
            && PropositionContentIR::compile(&self.source_proposition)
                .bindings
                .contains(&self.binding)
    }
}
