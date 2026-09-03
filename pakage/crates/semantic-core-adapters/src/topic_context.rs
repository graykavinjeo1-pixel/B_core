//! Hash-bound topic-local discourse state.
//!
//! The graph records which dialogue-local focus, pending question, and typed
//! discourse referents belong to each live topic. It is a routing prior only:
//! it cannot establish semantic truth or authorize execution.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const TOPIC_CONTEXT_GRAPH_SCHEMA: &str = "B_CORE_TOPIC_CONTEXT_GRAPH_IR_1";
pub const MAX_TOPIC_CONTEXTS: usize = 16;
pub const MAX_TOPIC_CONTEXT_TRANSITIONS: usize = 32;
pub const MAX_TOPIC_CONTEXT_REFERENTS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TopicContextStatusIR {
    Active,
    Suspended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TopicContextTransitionKindIR {
    Activate,
    Continue,
    Resume,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicContextIR {
    pub topic_id: String,
    pub topic_sha256: String,
    pub status: TopicContextStatusIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_focus_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_question_id: Option<String>,
    #[serde(default)]
    pub discourse_referent_ids: Vec<String>,
    pub introduced_turn: u64,
    pub last_activated_turn: u64,
    pub last_updated_turn: u64,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
    pub context_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicContextTransitionIR {
    pub transition_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_topic_id: Option<String>,
    pub resulting_topic_id: String,
    pub kind: TopicContextTransitionKindIR,
    pub turn_index: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restored_focus_id: Option<String>,
    pub evidence: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
    pub transition_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicContextGraphIR {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_topic_id: Option<String>,
    pub contexts: Vec<TopicContextIR>,
    pub transitions: Vec<TopicContextTransitionIR>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
    pub graph_sha256: String,
}

impl Default for TopicContextGraphIR {
    fn default() -> Self {
        let mut graph = Self {
            schema: TOPIC_CONTEXT_GRAPH_SCHEMA.to_string(),
            active_topic_id: None,
            contexts: Vec::new(),
            transitions: Vec::new(),
            semantic_authority: false,
            external_execution_authorized: false,
            graph_sha256: String::new(),
        };
        graph.reseal();
        graph
    }
}

impl TopicContextGraphIR {
    pub fn active(&self) -> Option<&TopicContextIR> {
        let active_topic_id = self.active_topic_id.as_deref()?;
        self.contexts
            .iter()
            .find(|context| context.topic_id == active_topic_id)
    }

    /// Captures the outgoing topic's focus, activates the target, and returns
    /// a previously suspended target focus when one can be restored exactly.
    pub fn activate(
        &mut self,
        topic_id: &str,
        topic_sha256: &str,
        turn_index: u64,
        outgoing_focus_id: Option<&str>,
        live_topic_ids: &[String],
        evidence: &[String],
    ) -> Option<String> {
        if topic_id.trim().is_empty() || topic_sha256.trim().is_empty() || turn_index == 0 {
            return None;
        }
        let prior_topic_id = self.active_topic_id.clone();
        if let Some(prior) = prior_topic_id.as_deref() {
            if let Some(context) = self
                .contexts
                .iter_mut()
                .find(|context| context.topic_id == prior)
            {
                if let Some(focus_id) = outgoing_focus_id.filter(|id| !id.trim().is_empty()) {
                    context.current_focus_id = Some(focus_id.to_string());
                }
                context.status = TopicContextStatusIR::Suspended;
                context.last_updated_turn = turn_index;
                context.context_sha256 = topic_context_sha256(context);
            }
        }
        let existing_focus = self
            .contexts
            .iter()
            .find(|context| context.topic_id == topic_id)
            .and_then(|context| context.current_focus_id.clone());
        let existing = self
            .contexts
            .iter()
            .any(|context| context.topic_id == topic_id);
        let introduced_turn = self
            .contexts
            .iter()
            .find(|context| context.topic_id == topic_id)
            .map_or(turn_index, |context| context.introduced_turn);
        let prior_question = self
            .contexts
            .iter()
            .find(|context| context.topic_id == topic_id)
            .and_then(|context| context.pending_question_id.clone());
        let prior_referents = self
            .contexts
            .iter()
            .find(|context| context.topic_id == topic_id)
            .map_or_else(Vec::new, |context| context.discourse_referent_ids.clone());
        self.contexts.retain(|context| context.topic_id != topic_id);
        let mut context = TopicContextIR {
            topic_id: topic_id.to_string(),
            topic_sha256: topic_sha256.to_string(),
            status: TopicContextStatusIR::Active,
            current_focus_id: existing_focus.clone(),
            pending_question_id: prior_question,
            discourse_referent_ids: prior_referents,
            introduced_turn,
            last_activated_turn: turn_index,
            last_updated_turn: turn_index,
            semantic_authority: false,
            external_execution_authorized: false,
            context_sha256: String::new(),
        };
        context.context_sha256 = topic_context_sha256(&context);
        self.contexts.push(context);
        self.active_topic_id = Some(topic_id.to_string());
        let kind = if prior_topic_id.as_deref() == Some(topic_id) {
            TopicContextTransitionKindIR::Continue
        } else if existing {
            TopicContextTransitionKindIR::Resume
        } else {
            TopicContextTransitionKindIR::Activate
        };
        let restored_focus_id = (kind == TopicContextTransitionKindIR::Resume)
            .then_some(existing_focus)
            .flatten();
        let mut transition_evidence = evidence.to_vec();
        transition_evidence.push("TOPIC_LOCAL_STATE_ROUTING:true".to_string());
        transition_evidence.push("SEMANTIC_AUTHORITY:false".to_string());
        transition_evidence.push("EXTERNAL_EXECUTION_AUTHORIZED:false".to_string());
        transition_evidence.sort();
        transition_evidence.dedup();
        let mut transition = TopicContextTransitionIR {
            transition_id: format!(
                "TOPIC-CONTEXT-TRANSITION-{turn_index:06}-{:02}",
                self.transitions.len() + 1
            ),
            prior_topic_id,
            resulting_topic_id: topic_id.to_string(),
            kind,
            turn_index,
            restored_focus_id: restored_focus_id.clone(),
            evidence: transition_evidence,
            semantic_authority: false,
            external_execution_authorized: false,
            transition_sha256: String::new(),
        };
        transition.transition_sha256 = topic_context_transition_sha256(&transition);
        self.transitions.push(transition);
        self.prune(live_topic_ids);
        self.reseal();
        restored_focus_id
    }

    #[allow(clippy::too_many_arguments)]
    pub fn refresh_active(
        &mut self,
        topic_id: &str,
        topic_sha256: &str,
        current_focus_id: Option<&str>,
        pending_question_id: Option<&str>,
        discourse_referent_ids: &[String],
        turn_index: u64,
        live_topic_ids: &[String],
    ) {
        let Some(context) = self
            .contexts
            .iter_mut()
            .find(|context| context.topic_id == topic_id)
        else {
            return;
        };
        context.topic_sha256 = topic_sha256.to_string();
        context.status = TopicContextStatusIR::Active;
        context.current_focus_id = current_focus_id.map(ToString::to_string);
        context.pending_question_id = pending_question_id.map(ToString::to_string);
        context.discourse_referent_ids = discourse_referent_ids
            .iter()
            .filter(|id| !id.trim().is_empty())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if context.discourse_referent_ids.len() > MAX_TOPIC_CONTEXT_REFERENTS {
            let excess = context.discourse_referent_ids.len() - MAX_TOPIC_CONTEXT_REFERENTS;
            context.discourse_referent_ids.drain(..excess);
        }
        context.last_updated_turn = turn_index;
        context.context_sha256 = topic_context_sha256(context);
        self.active_topic_id = Some(topic_id.to_string());
        for other in self
            .contexts
            .iter_mut()
            .filter(|context| context.topic_id != topic_id)
        {
            other.status = TopicContextStatusIR::Suspended;
            other.context_sha256 = topic_context_sha256(other);
        }
        self.prune(live_topic_ids);
        self.reseal();
    }

    pub fn retain_live_discourse_referents(
        &mut self,
        live_referent_ids: &BTreeSet<String>,
        turn_index: u64,
    ) {
        let mut changed = false;
        for context in &mut self.contexts {
            let prior_len = context.discourse_referent_ids.len();
            context
                .discourse_referent_ids
                .retain(|referent_id| live_referent_ids.contains(referent_id));
            if context.discourse_referent_ids.len() != prior_len {
                context.last_updated_turn = turn_index;
                context.context_sha256 = topic_context_sha256(context);
                changed = true;
            }
        }
        if changed {
            self.reseal();
        }
    }

    pub fn validate(&self, completed_turns: u64) -> bool {
        if self.schema != TOPIC_CONTEXT_GRAPH_SCHEMA
            || self.contexts.len() > MAX_TOPIC_CONTEXTS
            || self.transitions.len() > MAX_TOPIC_CONTEXT_TRANSITIONS
            || self.semantic_authority
            || self.external_execution_authorized
            || self.graph_sha256 != topic_context_graph_sha256(self)
        {
            return false;
        }
        let context_ids = self
            .contexts
            .iter()
            .map(|context| context.topic_id.as_str())
            .collect::<BTreeSet<_>>();
        let transition_ids = self
            .transitions
            .iter()
            .map(|transition| transition.transition_id.as_str())
            .collect::<BTreeSet<_>>();
        let active = self
            .contexts
            .iter()
            .filter(|context| context.status == TopicContextStatusIR::Active)
            .collect::<Vec<_>>();
        if context_ids.len() != self.contexts.len()
            || transition_ids.len() != self.transitions.len()
            || self.active_topic_id.is_some() != (active.len() == 1)
            || self.active_topic_id.as_deref()
                != active.first().map(|context| context.topic_id.as_str())
        {
            return false;
        }
        self.contexts.iter().all(|context| {
            !context.topic_id.trim().is_empty()
                && context.topic_sha256.len() == 64
                && context
                    .topic_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                && context.discourse_referent_ids.len() <= MAX_TOPIC_CONTEXT_REFERENTS
                && context
                    .discourse_referent_ids
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    == context.discourse_referent_ids.len()
                && context.introduced_turn > 0
                && context.last_activated_turn >= context.introduced_turn
                && context.last_updated_turn >= context.introduced_turn
                && context.last_activated_turn <= completed_turns
                && context.last_updated_turn <= completed_turns
                && !context.semantic_authority
                && !context.external_execution_authorized
                && context.context_sha256 == topic_context_sha256(context)
        }) && self.transitions.iter().all(|transition| {
            !transition.transition_id.trim().is_empty()
                && context_ids.contains(transition.resulting_topic_id.as_str())
                && transition
                    .prior_topic_id
                    .as_deref()
                    .is_none_or(|prior| context_ids.contains(prior))
                && transition.turn_index > 0
                && transition.turn_index <= completed_turns
                && !transition.evidence.is_empty()
                && !transition.semantic_authority
                && !transition.external_execution_authorized
                && transition.transition_sha256 == topic_context_transition_sha256(transition)
        })
    }

    fn prune(&mut self, live_topic_ids: &[String]) {
        let live = live_topic_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        self.contexts
            .retain(|context| live.contains(context.topic_id.as_str()));
        self.contexts.sort_by(|left, right| {
            left.status
                .cmp(&right.status)
                .then_with(|| right.last_updated_turn.cmp(&left.last_updated_turn))
                .then_with(|| left.topic_id.cmp(&right.topic_id))
        });
        self.contexts.truncate(MAX_TOPIC_CONTEXTS);
        let retained = self
            .contexts
            .iter()
            .map(|context| context.topic_id.as_str())
            .collect::<BTreeSet<_>>();
        self.transitions.retain(|transition| {
            retained.contains(transition.resulting_topic_id.as_str())
                && transition
                    .prior_topic_id
                    .as_deref()
                    .is_none_or(|prior| retained.contains(prior))
        });
        if self.transitions.len() > MAX_TOPIC_CONTEXT_TRANSITIONS {
            let excess = self.transitions.len() - MAX_TOPIC_CONTEXT_TRANSITIONS;
            self.transitions.drain(..excess);
        }
        if self
            .active_topic_id
            .as_deref()
            .is_some_and(|active| !retained.contains(active))
        {
            self.active_topic_id = None;
        }
    }

    fn reseal(&mut self) {
        self.graph_sha256 = topic_context_graph_sha256(self);
    }
}

pub fn topic_context_sha256(context: &TopicContextIR) -> String {
    let bytes = serde_json::to_vec(&(
        "B_CORE_TOPIC_CONTEXT_IR_1",
        &context.topic_id,
        &context.topic_sha256,
        context.status,
        &context.current_focus_id,
        &context.pending_question_id,
        &context.discourse_referent_ids,
        context.introduced_turn,
        context.last_activated_turn,
        context.last_updated_turn,
        context.semantic_authority,
        context.external_execution_authorized,
    ))
    .expect("bounded topic context serializes");
    format!("{:x}", Sha256::digest(bytes))
}

pub fn topic_context_transition_sha256(transition: &TopicContextTransitionIR) -> String {
    let bytes = serde_json::to_vec(&(
        "B_CORE_TOPIC_CONTEXT_TRANSITION_IR_1",
        &transition.transition_id,
        &transition.prior_topic_id,
        &transition.resulting_topic_id,
        transition.kind,
        transition.turn_index,
        &transition.restored_focus_id,
        &transition.evidence,
        transition.semantic_authority,
        transition.external_execution_authorized,
    ))
    .expect("bounded topic context transition serializes");
    format!("{:x}", Sha256::digest(bytes))
}

pub fn topic_context_graph_sha256(graph: &TopicContextGraphIR) -> String {
    let bytes = serde_json::to_vec(&(
        &graph.schema,
        &graph.active_topic_id,
        &graph.contexts,
        &graph.transitions,
        graph.semantic_authority,
        graph.external_execution_authorized,
    ))
    .expect("bounded topic context graph serializes");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic_hash(seed: &str) -> String {
        format!("{:x}", Sha256::digest(seed.as_bytes()))
    }

    #[test]
    fn suspended_focus_is_restored_without_authority() {
        let mut graph = TopicContextGraphIR::default();
        let cache = "TOPIC-CACHE".to_string();
        let queue = "TOPIC-QUEUE".to_string();
        graph.activate(
            &cache,
            &topic_hash("cache"),
            1,
            None,
            std::slice::from_ref(&cache),
            &["EXPLICIT_TOPIC".to_string()],
        );
        graph.refresh_active(
            &cache,
            &topic_hash("cache"),
            Some("FOCUS-FILE"),
            None,
            &["DREF-FILE".to_string()],
            2,
            std::slice::from_ref(&cache),
        );
        graph.activate(
            &queue,
            &topic_hash("queue"),
            3,
            Some("FOCUS-FILE"),
            &[queue.clone(), cache.clone()],
            &["EXPLICIT_TOPIC".to_string()],
        );
        graph.refresh_active(
            &queue,
            &topic_hash("queue"),
            Some("FOCUS-FOLDER"),
            None,
            &["DREF-FOLDER".to_string()],
            4,
            &[queue.clone(), cache.clone()],
        );
        let restored = graph.activate(
            &cache,
            &topic_hash("cache-resumed"),
            5,
            Some("FOCUS-FOLDER"),
            &[cache.clone(), queue],
            &["EXPLICIT_RETURN".to_string()],
        );
        assert_eq!(restored.as_deref(), Some("FOCUS-FILE"));
        assert_eq!(
            graph.transitions.last().map(|transition| transition.kind),
            Some(TopicContextTransitionKindIR::Resume)
        );
        assert!(graph.validate(5));
        assert!(!graph.semantic_authority);
        assert!(!graph.external_execution_authorized);
    }

    #[test]
    fn retired_referents_are_pruned_from_suspended_topic_contexts() {
        let mut graph = TopicContextGraphIR::default();
        let cache = "TOPIC-CACHE".to_string();
        let queue = "TOPIC-QUEUE".to_string();
        graph.activate(
            &cache,
            &topic_hash("cache"),
            1,
            None,
            std::slice::from_ref(&cache),
            &[],
        );
        graph.refresh_active(
            &cache,
            &topic_hash("cache"),
            Some("FOCUS-CACHE"),
            None,
            &["DREF-CACHE".to_string()],
            1,
            std::slice::from_ref(&cache),
        );
        graph.activate(
            &queue,
            &topic_hash("queue"),
            2,
            Some("FOCUS-CACHE"),
            &[queue.clone(), cache.clone()],
            &[],
        );
        graph.refresh_active(
            &queue,
            &topic_hash("queue"),
            Some("FOCUS-QUEUE"),
            None,
            &["DREF-QUEUE".to_string()],
            2,
            &[queue.clone(), cache],
        );

        graph.retain_live_discourse_referents(&BTreeSet::from(["DREF-QUEUE".to_string()]), 3);

        assert!(graph
            .contexts
            .iter()
            .find(|context| context.topic_id == "TOPIC-CACHE")
            .is_some_and(|context| context.discourse_referent_ids.is_empty()));
        assert_eq!(
            graph
                .active()
                .map(|context| context.discourse_referent_ids.as_slice()),
            Some(["DREF-QUEUE".to_string()].as_slice())
        );
        assert!(graph.validate(3));
    }
}
