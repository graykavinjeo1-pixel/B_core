//! Bounded, tamper-evident memory for multi-turn pragmatic state.
//!
//! This memory stores task and decision frames produced by the language
//! adapter. It is not semantic concept authority and cannot mutate the world
//! model. Its purpose is to let later elliptical turns recover a typed open
//! task or an unresolved cost/benefit gate without replaying the full chat.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::conversation::ConversationTurnRequestIR;
use crate::pragmatics::{PragmaticContextIR, PragmaticInterpretationIR, SpeechActIR};

pub const PRAGMATIC_MEMORY_STATE_SCHEMA: &str = "B_CORE_PRAGMATIC_MEMORY_STATE_IR_2";
const MAX_TASK_FRAMES: usize = 8;
const MAX_TOPIC_PENDING_GATES: usize = 8;
const MAX_TURN_SUMMARIES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PendingGateStatusIR {
    AwaitingEvidence,
    SuspendedByUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PragmaticTaskFrameIR {
    pub frame_id: String,
    pub task: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_id: Option<String>,
    pub introduced_turn: u64,
    pub last_referenced_turn: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingContinuationGateIR {
    pub task: String,
    pub required_benefit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_id: Option<String>,
    pub source_turn: u64,
    pub last_referenced_turn: u64,
    pub status: PendingGateStatusIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PragmaticTurnSummaryIR {
    pub turn_index: u64,
    pub speech_act: SpeechActIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_benefit: Option<String>,
    pub unresolved_binding_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PragmaticMemoryStateIR {
    pub schema: String,
    pub conversation_id: String,
    pub completed_turns: u64,
    pub task_frames: Vec<PragmaticTaskFrameIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_continuation_gate: Option<PendingContinuationGateIR>,
    /// Hash-bound suspended gates keyed by discourse topic. This remains
    /// language-adapter memory and never acquires semantic authority.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topic_pending_continuation_gates: Vec<PendingContinuationGateIR>,
    pub recent_turns: Vec<PragmaticTurnSummaryIR>,
    pub state_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PragmaticMemoryError {
    InvalidRequest,
    TurnOrder,
    InvalidState,
}

#[derive(Debug, Clone, Default)]
pub struct PragmaticMemory {
    states: BTreeMap<String, PragmaticMemoryStateIR>,
}

impl PragmaticMemory {
    pub fn state(&self, conversation_id: &str) -> Option<&PragmaticMemoryStateIR> {
        self.states.get(conversation_id)
    }

    pub fn context(&self, conversation_id: &str) -> PragmaticContextIR {
        self.context_in_topic(conversation_id, None)
    }

    /// Builds context for the currently restored discourse topic. A supplied
    /// topic is a strict scope: unrelated global recency cannot leak into it.
    pub fn context_in_topic(
        &self,
        conversation_id: &str,
        topic_id: Option<&str>,
    ) -> PragmaticContextIR {
        let Some(state) = self.states.get(conversation_id) else {
            return PragmaticContextIR::default();
        };
        let pending = pending_gate_for_topic(state, topic_id);
        let current_task = pending.map(|gate| gate.task.clone()).or_else(|| {
            state
                .task_frames
                .iter()
                .find(|frame| match topic_id {
                    Some(topic_id) => frame.topic_id.as_deref() == Some(topic_id),
                    None => true,
                })
                .map(|frame| frame.task.clone())
        });
        PragmaticContextIR {
            current_task,
            active_subject: None,
            pending_required_benefit: pending.map(|gate| gate.required_benefit.clone()),
            pending_gate_suspended: pending
                .is_some_and(|gate| gate.status == PendingGateStatusIR::SuspendedByUser),
            active_goals: Vec::new(),
            pending_deferred_commitments: Vec::new(),
            recent_subjects: Vec::new(),
        }
    }

    pub fn pending_gate_in_topic(
        &self,
        conversation_id: &str,
        topic_id: Option<&str>,
    ) -> Option<PendingContinuationGateIR> {
        self.states
            .get(conversation_id)
            .and_then(|state| pending_gate_for_topic(state, topic_id))
            .cloned()
    }

    pub fn validate_turn_order(
        &self,
        request: &ConversationTurnRequestIR,
    ) -> Result<(), PragmaticMemoryError> {
        if request.conversation_id.trim().is_empty() || request.turn_index == 0 {
            return Err(PragmaticMemoryError::InvalidRequest);
        }
        let expected = self
            .states
            .get(&request.conversation_id)
            .map_or(1, |state| state.completed_turns.saturating_add(1));
        if request.turn_index != expected {
            return Err(PragmaticMemoryError::TurnOrder);
        }
        Ok(())
    }

    pub fn commit_turn(
        &mut self,
        request: &ConversationTurnRequestIR,
        interpretation: &PragmaticInterpretationIR,
    ) -> Result<PragmaticMemoryStateIR, PragmaticMemoryError> {
        self.commit_turn_in_topic(request, interpretation, None)
    }

    pub fn commit_turn_in_topic(
        &mut self,
        request: &ConversationTurnRequestIR,
        interpretation: &PragmaticInterpretationIR,
        topic_id: Option<&str>,
    ) -> Result<PragmaticMemoryStateIR, PragmaticMemoryError> {
        self.validate_turn_order(request)?;
        let state = self
            .states
            .entry(request.conversation_id.clone())
            .or_insert_with(|| empty_state(&request.conversation_id));

        if let Some(task) = interpretation.inferred_current_task.as_deref() {
            let topic_id = topic_id.map(str::to_string);
            let introduced_turn = state
                .task_frames
                .iter()
                .find(|frame| frame.task == task && frame.topic_id == topic_id)
                .map_or(request.turn_index, |frame| frame.introduced_turn);
            state
                .task_frames
                .retain(|frame| frame.task != task || frame.topic_id != topic_id);
            state.task_frames.insert(
                0,
                PragmaticTaskFrameIR {
                    frame_id: task_frame_id(task, topic_id.as_deref()),
                    task: task.to_string(),
                    topic_id,
                    introduced_turn,
                    last_referenced_turn: request.turn_index,
                },
            );
            state.task_frames.truncate(MAX_TASK_FRAMES);
        }

        if let Some(gate) = &interpretation.continuation_gate {
            let topic_id = topic_id.map(str::to_string);
            let prior = pending_gate_for_topic(state, topic_id.as_deref()).cloned();
            let next = PendingContinuationGateIR {
                task: gate.current_task.clone(),
                required_benefit: gate.required_benefit.clone(),
                topic_id: topic_id.clone(),
                source_turn: prior
                    .as_ref()
                    .filter(|prior| {
                        prior.task == gate.current_task
                            && prior.required_benefit == gate.required_benefit
                    })
                    .map_or(request.turn_index, |prior| prior.source_turn),
                last_referenced_turn: request.turn_index,
                status: PendingGateStatusIR::AwaitingEvidence,
            };
            if let Some(topic_id) = topic_id.as_deref() {
                state
                    .topic_pending_continuation_gates
                    .retain(|prior| prior.topic_id.as_deref() != Some(topic_id));
                state
                    .topic_pending_continuation_gates
                    .insert(0, next.clone());
                state
                    .topic_pending_continuation_gates
                    .truncate(MAX_TOPIC_PENDING_GATES);
            }
            state.pending_continuation_gate = Some(next);
        } else if interpretation.speech_act == SpeechActIR::Reject {
            if let Some(gate) = pending_gate_for_topic_mut(state, topic_id) {
                gate.status = PendingGateStatusIR::SuspendedByUser;
                gate.last_referenced_turn = request.turn_index;
                state.pending_continuation_gate = Some(gate.clone());
            }
        } else if topic_id.is_some() {
            state.pending_continuation_gate = pending_gate_for_topic(state, topic_id).cloned();
        }

        state.recent_turns.push(PragmaticTurnSummaryIR {
            turn_index: request.turn_index,
            speech_act: interpretation.speech_act,
            task: interpretation.inferred_current_task.clone(),
            required_benefit: interpretation
                .continuation_gate
                .as_ref()
                .map(|gate| gate.required_benefit.clone()),
            unresolved_binding_count: interpretation.unresolved_bindings.len(),
        });
        if state.recent_turns.len() > MAX_TURN_SUMMARIES {
            let remove = state.recent_turns.len() - MAX_TURN_SUMMARIES;
            state.recent_turns.drain(..remove);
        }
        state.completed_turns = request.turn_index;
        state.state_sha256 = state_hash(state)?;
        validate_pragmatic_memory_state(state)?;
        Ok(state.clone())
    }
}

fn empty_state(conversation_id: &str) -> PragmaticMemoryStateIR {
    let mut state = PragmaticMemoryStateIR {
        schema: PRAGMATIC_MEMORY_STATE_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        completed_turns: 0,
        task_frames: Vec::new(),
        pending_continuation_gate: None,
        topic_pending_continuation_gates: Vec::new(),
        recent_turns: Vec::new(),
        state_sha256: String::new(),
    };
    state.state_sha256 = state_hash(&state).expect("empty pragmatic state serializes");
    state
}

fn task_frame_id(task: &str, topic_id: Option<&str>) -> String {
    let digest = Sha256::digest(
        format!(
            "{}\u{1f}{}",
            task.trim().to_lowercase(),
            topic_id.unwrap_or_default()
        )
        .as_bytes(),
    );
    format!(
        "TASK-{:02X}{:02X}{:02X}{:02X}",
        digest[0], digest[1], digest[2], digest[3]
    )
}

fn state_hash(state: &PragmaticMemoryStateIR) -> Result<String, PragmaticMemoryError> {
    let bytes = serde_json::to_vec(&(
        &state.schema,
        &state.conversation_id,
        state.completed_turns,
        &state.task_frames,
        &state.pending_continuation_gate,
        &state.topic_pending_continuation_gates,
        &state.recent_turns,
    ))
    .map_err(|_| PragmaticMemoryError::InvalidState)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn validate_pragmatic_memory_state(
    state: &PragmaticMemoryStateIR,
) -> Result<(), PragmaticMemoryError> {
    let unique_tasks = state
        .task_frames
        .iter()
        .map(|frame| (&frame.task, &frame.topic_id))
        .collect::<BTreeSet<_>>();
    let unique_gate_topics = state
        .topic_pending_continuation_gates
        .iter()
        .filter_map(|gate| gate.topic_id.as_deref())
        .collect::<BTreeSet<_>>();
    if state.schema != PRAGMATIC_MEMORY_STATE_SCHEMA
        || state.conversation_id.trim().is_empty()
        || state.task_frames.len() > MAX_TASK_FRAMES
        || unique_tasks.len() != state.task_frames.len()
        || state.topic_pending_continuation_gates.len() > MAX_TOPIC_PENDING_GATES
        || unique_gate_topics.len() != state.topic_pending_continuation_gates.len()
        || state
            .topic_pending_continuation_gates
            .iter()
            .any(|gate| gate.topic_id.as_deref().is_none_or(str::is_empty))
        || state.recent_turns.len() > MAX_TURN_SUMMARIES
        || state.state_sha256.len() != 64
        || state.state_sha256 != state_hash(state)?
    {
        return Err(PragmaticMemoryError::InvalidState);
    }
    Ok(())
}

fn pending_gate_for_topic<'a>(
    state: &'a PragmaticMemoryStateIR,
    topic_id: Option<&str>,
) -> Option<&'a PendingContinuationGateIR> {
    match topic_id {
        Some(topic_id) => state
            .topic_pending_continuation_gates
            .iter()
            .find(|gate| gate.topic_id.as_deref() == Some(topic_id))
            .or_else(|| {
                state
                    .pending_continuation_gate
                    .as_ref()
                    .filter(|gate| gate.topic_id.as_deref() == Some(topic_id))
            }),
        None => state.pending_continuation_gate.as_ref(),
    }
}

fn pending_gate_for_topic_mut<'a>(
    state: &'a mut PragmaticMemoryStateIR,
    topic_id: Option<&str>,
) -> Option<&'a mut PendingContinuationGateIR> {
    match topic_id {
        Some(topic_id) => state
            .topic_pending_continuation_gates
            .iter_mut()
            .find(|gate| gate.topic_id.as_deref() == Some(topic_id)),
        None => state.pending_continuation_gate.as_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::{ConversationInputModalityIR, CONVERSATION_TURN_REQUEST_SCHEMA};
    use crate::language_knowledge::LanguageCodeIR;
    use crate::pragmatics::PragmaticReasoner;

    fn request(turn_index: u64, text: &str) -> ConversationTurnRequestIR {
        ConversationTurnRequestIR {
            schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
            conversation_id: "MEMORY-TEST".to_string(),
            turn_index,
            request_id: format!("MEMORY-TEST-{turn_index}"),
            modality: ConversationInputModalityIR::Text,
            raw_text: text.to_string(),
            input_confidence_millis: 1_000,
            alternatives: Vec::new(),
            output_language: Some(LanguageCodeIR::Korean),
            context_tags: Vec::new(),
            max_plan_steps: 12,
        }
    }

    #[test]
    fn pending_gate_becomes_compact_context_for_later_ellipsis() {
        let mut memory = PragmaticMemory::default();
        let first = request(
            1,
            "리팩터링은 힘들다. 리팩터링을 하면 장애가 줄어든다. 그 정도면 계속할 만하다.",
        );
        let interpretation =
            PragmaticReasoner.interpret(&first.raw_text, &PragmaticContextIR::default());
        memory
            .commit_turn(&first, &interpretation)
            .expect("first turn");
        let context = memory.context("MEMORY-TEST");
        assert_eq!(context.current_task.as_deref(), Some("리팩터링"));
        assert!(context
            .pending_required_benefit
            .as_deref()
            .is_some_and(|benefit| benefit.contains("장애")));
    }

    #[test]
    fn state_is_bounded_turn_ordered_and_tamper_evident() {
        let mut memory = PragmaticMemory::default();
        for turn in 1..=20 {
            let request = request(turn, "로그가 왜 비었는지 궁금하네");
            let interpretation =
                PragmaticReasoner.interpret(&request.raw_text, &PragmaticContextIR::default());
            memory
                .commit_turn(&request, &interpretation)
                .expect("ordered turn");
        }
        let state = memory.state("MEMORY-TEST").expect("state");
        assert_eq!(state.recent_turns.len(), MAX_TURN_SUMMARIES);
        validate_pragmatic_memory_state(state).expect("valid state");
        let mut tampered = state.clone();
        tampered.completed_turns += 1;
        assert_eq!(
            validate_pragmatic_memory_state(&tampered),
            Err(PragmaticMemoryError::InvalidState)
        );
    }

    #[test]
    fn topic_scopes_restore_independent_tasks_and_pending_gates() {
        let mut memory = PragmaticMemory::default();
        let first = request(
            1,
            "리팩터링은 힘들다. 리팩터링을 하면 실제 장애가 줄어든다. 그 정도면 계속할 만하다.",
        );
        let first_interpretation =
            PragmaticReasoner.interpret(&first.raw_text, &PragmaticContextIR::default());
        memory
            .commit_turn_in_topic(&first, &first_interpretation, Some("TOPIC-CACHE"))
            .expect("cache gate");

        let second = request(
            2,
            "마이그레이션은 어렵다. 마이그레이션을 하면 실제 커버리지가 넓어진다. 그 정도면 계속할 만하다.",
        );
        let second_interpretation =
            PragmaticReasoner.interpret(&second.raw_text, &PragmaticContextIR::default());
        memory
            .commit_turn_in_topic(&second, &second_interpretation, Some("TOPIC-QUEUE"))
            .expect("queue gate");

        let cache = memory.context_in_topic("MEMORY-TEST", Some("TOPIC-CACHE"));
        let queue = memory.context_in_topic("MEMORY-TEST", Some("TOPIC-QUEUE"));
        assert_eq!(cache.current_task.as_deref(), Some("리팩터링"));
        assert!(cache
            .pending_required_benefit
            .as_deref()
            .is_some_and(|benefit| benefit.contains("장애")));
        assert_eq!(queue.current_task.as_deref(), Some("마이그레이션"));
        assert!(queue
            .pending_required_benefit
            .as_deref()
            .is_some_and(|benefit| benefit.contains("커버리지")));
        assert_eq!(
            memory
                .state("MEMORY-TEST")
                .expect("state")
                .topic_pending_continuation_gates
                .len(),
            2
        );
    }

    #[test]
    fn unseen_explicit_topic_does_not_fall_back_to_global_recency() {
        let mut memory = PragmaticMemory::default();
        let first = request(1, "현재 작업은 리팩터링이다.");
        let interpretation =
            PragmaticReasoner.interpret(&first.raw_text, &PragmaticContextIR::default());
        memory
            .commit_turn_in_topic(&first, &interpretation, Some("TOPIC-CACHE"))
            .expect("cache task");

        let unseen = memory.context_in_topic("MEMORY-TEST", Some("TOPIC-REPORT"));
        assert!(unseen.current_task.is_none());
        assert!(unseen.pending_required_benefit.is_none());
    }
}
