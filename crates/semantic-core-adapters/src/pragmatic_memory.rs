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

pub const PRAGMATIC_MEMORY_STATE_SCHEMA: &str = "B_CORE_PRAGMATIC_MEMORY_STATE_IR_1";
const MAX_TASK_FRAMES: usize = 8;
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
    pub introduced_turn: u64,
    pub last_referenced_turn: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingContinuationGateIR {
    pub task: String,
    pub required_benefit: String,
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
        let Some(state) = self.states.get(conversation_id) else {
            return PragmaticContextIR::default();
        };
        let pending = state.pending_continuation_gate.as_ref();
        let current_task = pending
            .map(|gate| gate.task.clone())
            .or_else(|| state.task_frames.first().map(|frame| frame.task.clone()));
        PragmaticContextIR {
            current_task,
            active_subject: None,
            pending_required_benefit: pending.map(|gate| gate.required_benefit.clone()),
            pending_gate_suspended: pending
                .is_some_and(|gate| gate.status == PendingGateStatusIR::SuspendedByUser),
        }
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
        self.validate_turn_order(request)?;
        let state = self
            .states
            .entry(request.conversation_id.clone())
            .or_insert_with(|| empty_state(&request.conversation_id));

        if let Some(task) = interpretation.inferred_current_task.as_deref() {
            let introduced_turn = state
                .task_frames
                .iter()
                .find(|frame| frame.task == task)
                .map_or(request.turn_index, |frame| frame.introduced_turn);
            state.task_frames.retain(|frame| frame.task != task);
            state.task_frames.insert(
                0,
                PragmaticTaskFrameIR {
                    frame_id: task_frame_id(task),
                    task: task.to_string(),
                    introduced_turn,
                    last_referenced_turn: request.turn_index,
                },
            );
            state.task_frames.truncate(MAX_TASK_FRAMES);
        }

        if let Some(gate) = &interpretation.continuation_gate {
            state.pending_continuation_gate = Some(PendingContinuationGateIR {
                task: gate.current_task.clone(),
                required_benefit: gate.required_benefit.clone(),
                source_turn: state
                    .pending_continuation_gate
                    .as_ref()
                    .filter(|prior| {
                        prior.task == gate.current_task
                            && prior.required_benefit == gate.required_benefit
                    })
                    .map_or(request.turn_index, |prior| prior.source_turn),
                last_referenced_turn: request.turn_index,
                status: PendingGateStatusIR::AwaitingEvidence,
            });
        } else if interpretation.speech_act == SpeechActIR::Reject {
            if let Some(gate) = &mut state.pending_continuation_gate {
                gate.status = PendingGateStatusIR::SuspendedByUser;
                gate.last_referenced_turn = request.turn_index;
            }
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
        recent_turns: Vec::new(),
        state_sha256: String::new(),
    };
    state.state_sha256 = state_hash(&state).expect("empty pragmatic state serializes");
    state
}

fn task_frame_id(task: &str) -> String {
    let digest = Sha256::digest(task.trim().to_lowercase().as_bytes());
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
        .map(|frame| &frame.task)
        .collect::<BTreeSet<_>>();
    if state.schema != PRAGMATIC_MEMORY_STATE_SCHEMA
        || state.conversation_id.trim().is_empty()
        || state.task_frames.len() > MAX_TASK_FRAMES
        || unique_tasks.len() != state.task_frames.len()
        || state.recent_turns.len() > MAX_TURN_SUMMARIES
        || state.state_sha256.len() != 64
        || state.state_sha256 != state_hash(state)?
    {
        return Err(PragmaticMemoryError::InvalidState);
    }
    Ok(())
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
}
