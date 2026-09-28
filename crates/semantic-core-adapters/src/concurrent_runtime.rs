//! Bounded, sharded runtime for many independent B_Core conversations.
//!
//! Each conversation is deterministically routed to one mutable shard, so its
//! turn order remains serial while unrelated conversations can progress in
//! parallel.  The host admits a bounded number of simultaneous calls and does
//! not allocate one cognitive core per connected user.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

use serde::{Deserialize, Serialize};

use crate::cognitive::{
    CognitiveApi, CognitiveApiError, ConversationTurnResponseIR, ConversationTurnTimingIR,
};
use crate::conversation::ConversationTurnRequestIR;

pub const CONCURRENT_RUNTIME_STATS_SCHEMA: &str = "B_CORE_CONCURRENT_RUNTIME_STATS_IR_3";
/// Match the standalone HTTP worker bound so independently keyed
/// conversations do not serialize behind an avoidable shard lock.
pub const DEFAULT_CONVERSATION_SHARDS: usize = 32;
pub const DEFAULT_MAX_IN_FLIGHT: usize = 256;
pub const MAX_CONVERSATION_SHARDS: usize = 32;
pub const MAX_IN_FLIGHT_LIMIT: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConcurrentRuntimeError {
    InvalidConfiguration,
    Busy,
    ShardUnavailable,
    Cognitive(CognitiveApiError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcurrentRuntimeStatsIR {
    pub schema: String,
    pub shard_count: usize,
    pub max_in_flight: usize,
    pub current_in_flight: usize,
    pub peak_in_flight: usize,
    pub accepted_requests: usize,
    pub rejected_busy_requests: usize,
    /// Distinct conversation identifiers retaining any conversation-local
    /// state across all shards. The counter is updated while the owning shard
    /// lock is held, so monitoring does not lock every shard.
    pub active_conversations: usize,
}

/// Thread-safe conversation host. Global mutation operations intentionally do
/// not pass through this type: product conversation traffic is isolated from
/// knowledge/catalog administration.
pub struct ConcurrentConversationRuntime {
    shards: Vec<Mutex<CognitiveApi>>,
    max_in_flight: usize,
    current_in_flight: AtomicUsize,
    peak_in_flight: AtomicUsize,
    accepted_requests: AtomicUsize,
    rejected_busy_requests: AtomicUsize,
    active_conversations: AtomicUsize,
}

impl ConcurrentConversationRuntime {
    pub fn new(shard_count: usize, max_in_flight: usize) -> Result<Self, ConcurrentRuntimeError> {
        if shard_count == 0
            || shard_count > MAX_CONVERSATION_SHARDS
            || max_in_flight < shard_count
            || max_in_flight > MAX_IN_FLIGHT_LIMIT
        {
            return Err(ConcurrentRuntimeError::InvalidConfiguration);
        }
        // A constructed runtime is ready to serve: first requests never own
        // immutable lexical-pack parsing or form-index construction.
        crate::lexical_knowledge_pack::preload_builtin_pack();
        let shards = (0..shard_count)
            .map(|_| {
                CognitiveApi::new_embedded()
                    .map(Mutex::new)
                    .map_err(ConcurrentRuntimeError::Cognitive)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            shards,
            max_in_flight,
            current_in_flight: AtomicUsize::new(0),
            peak_in_flight: AtomicUsize::new(0),
            accepted_requests: AtomicUsize::new(0),
            rejected_busy_requests: AtomicUsize::new(0),
            active_conversations: AtomicUsize::new(0),
        })
    }

    pub fn standalone_default() -> Result<Self, ConcurrentRuntimeError> {
        Self::new(DEFAULT_CONVERSATION_SHARDS, DEFAULT_MAX_IN_FLIGHT)
    }

    pub fn process_conversation_turn(
        &self,
        request: &ConversationTurnRequestIR,
    ) -> Result<ConversationTurnResponseIR, ConcurrentRuntimeError> {
        let _admission = self.admit()?;
        let shard_index = stable_shard(&request.conversation_id, self.shards.len());
        let mut shard = self.shards[shard_index]
            .lock()
            .map_err(|_| ConcurrentRuntimeError::ShardUnavailable)?;
        let was_active = shard.conversation_is_active(&request.conversation_id);
        let result = shard.process_conversation_turn(request);
        sync_active_conversation_count(
            &self.active_conversations,
            was_active,
            shard.conversation_is_active(&request.conversation_id),
        );
        result.map_err(ConcurrentRuntimeError::Cognitive)
    }

    /// Releases one finished conversation from the shard that owns it. The
    /// same admission bound applies so cleanup cannot bypass host limits.
    pub fn close_conversation(&self, conversation_id: &str) -> Result<bool, ConcurrentRuntimeError> {
        let _admission = self.admit()?;
        let shard_index = stable_shard(conversation_id, self.shards.len());
        let mut shard = self.shards[shard_index]
            .lock()
            .map_err(|_| ConcurrentRuntimeError::ShardUnavailable)?;
        let was_active = shard.conversation_is_active(conversation_id);
        let result = shard.close_conversation(conversation_id);
        sync_active_conversation_count(
            &self.active_conversations,
            was_active,
            shard.conversation_is_active(conversation_id),
        );
        result.map_err(ConcurrentRuntimeError::Cognitive)
    }

    /// Runs the same bounded, sharded path while returning internal timing
    /// evidence. The timing starts after shard acquisition; callers that need
    /// end-to-end latency must measure admission and lock waiting separately.
    pub fn process_conversation_turn_profiled(
        &self,
        request: &ConversationTurnRequestIR,
    ) -> Result<(ConversationTurnResponseIR, ConversationTurnTimingIR), ConcurrentRuntimeError>
    {
        let _admission = self.admit()?;
        let shard_index = stable_shard(&request.conversation_id, self.shards.len());
        let mut shard = self.shards[shard_index]
            .lock()
            .map_err(|_| ConcurrentRuntimeError::ShardUnavailable)?;
        let was_active = shard.conversation_is_active(&request.conversation_id);
        let result = shard.process_conversation_turn_profiled(request);
        sync_active_conversation_count(
            &self.active_conversations,
            was_active,
            shard.conversation_is_active(&request.conversation_id),
        );
        result.map_err(ConcurrentRuntimeError::Cognitive)
    }

    pub fn stats(&self) -> ConcurrentRuntimeStatsIR {
        ConcurrentRuntimeStatsIR {
            schema: CONCURRENT_RUNTIME_STATS_SCHEMA.into(),
            shard_count: self.shards.len(),
            max_in_flight: self.max_in_flight,
            current_in_flight: self.current_in_flight.load(Ordering::Acquire),
            peak_in_flight: self.peak_in_flight.load(Ordering::Acquire),
            accepted_requests: self.accepted_requests.load(Ordering::Acquire),
            rejected_busy_requests: self.rejected_busy_requests.load(Ordering::Acquire),
            active_conversations: self.active_conversations.load(Ordering::Acquire),
        }
    }

    fn admit(&self) -> Result<AdmissionGuard<'_>, ConcurrentRuntimeError> {
        let mut current = self.current_in_flight.load(Ordering::Acquire);
        loop {
            if current >= self.max_in_flight {
                self.rejected_busy_requests.fetch_add(1, Ordering::AcqRel);
                return Err(ConcurrentRuntimeError::Busy);
            }
            match self.current_in_flight.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
        self.accepted_requests.fetch_add(1, Ordering::AcqRel);
        update_peak(&self.peak_in_flight, current + 1);
        Ok(AdmissionGuard { runtime: self })
    }
}

struct AdmissionGuard<'a> {
    runtime: &'a ConcurrentConversationRuntime,
}

impl Drop for AdmissionGuard<'_> {
    fn drop(&mut self) {
        self.runtime
            .current_in_flight
            .fetch_sub(1, Ordering::AcqRel);
    }
}

fn sync_active_conversation_count(counter: &AtomicUsize, was_active: bool, is_active: bool) {
    match (was_active, is_active) {
        (false, true) => {
            counter.fetch_add(1, Ordering::AcqRel);
        }
        (true, false) => {
            let previous = counter.fetch_sub(1, Ordering::AcqRel);
            debug_assert!(previous > 0, "active conversation counter underflow");
        }
        _ => {}
    }
}
fn update_peak(peak: &AtomicUsize, candidate: usize) {
    let mut observed = peak.load(Ordering::Acquire);
    while candidate > observed {
        match peak.compare_exchange_weak(observed, candidate, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(actual) => observed = actual,
        }
    }
}

fn stable_shard(conversation_id: &str, shard_count: usize) -> usize {
    // FNV-1a keeps routing stable across processes; DefaultHasher does not
    // promise that property.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in conversation_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash as usize) % shard_count
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};
    use std::thread;

    use super::*;
    use crate::conversation::{ConversationInputModalityIR, CONVERSATION_TURN_REQUEST_SCHEMA};
    use crate::language_knowledge::LanguageCodeIR;

    fn request(conversation: usize, turn_index: u64, text: &str) -> ConversationTurnRequestIR {
        ConversationTurnRequestIR {
            schema: CONVERSATION_TURN_REQUEST_SCHEMA.into(),
            conversation_id: format!("CONCURRENT-{conversation}"),
            turn_index,
            request_id: format!("CONCURRENT-{conversation}-{turn_index}"),
            modality: ConversationInputModalityIR::Text,
            raw_text: text.into(),
            input_confidence_millis: 1_000,
            alternatives: Vec::new(),
            output_language: Some(LanguageCodeIR::Korean),
            context_tags: Vec::new(),
            max_plan_steps: 12,
        }
    }

    #[test]
    fn two_hundred_independent_sessions_are_admitted_and_isolated() {
        const SESSIONS: usize = 200;
        let runtime = Arc::new(ConcurrentConversationRuntime::new(8, 256).unwrap());
        let barrier = Arc::new(Barrier::new(SESSIONS + 1));
        let handles = (0..SESSIONS)
            .map(|session| {
                let runtime = Arc::clone(&runtime);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    runtime.process_conversation_turn(&request(
                        session,
                        1,
                        "회의 시간이 오후 4시야.",
                    ))
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let responses = handles
            .into_iter()
            .map(|handle| handle.join().expect("worker thread"))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(responses.len(), SESSIONS);
        assert!(responses.iter().enumerate().all(|(session, response)| {
            response.conversation_id == format!("CONCURRENT-{session}") && response.turn_index == 1
        }));
        let stats = runtime.stats();
        assert_eq!(stats.accepted_requests, SESSIONS);
        assert_eq!(stats.rejected_busy_requests, 0);
        assert_eq!(stats.current_in_flight, 0);
        assert_eq!(stats.active_conversations, SESSIONS);
        assert!(stats.peak_in_flight > 1);
    }

    #[test]
    fn a_conversation_keeps_turn_order_inside_its_stable_shard() {
        let runtime = ConcurrentConversationRuntime::new(2, 8).unwrap();
        runtime
            .process_conversation_turn(&request(1, 1, "회의 시간이 오후 4시야."))
            .unwrap();
        let second = runtime
            .process_conversation_turn(&request(1, 2, "몇 시라고 했지?"))
            .unwrap();
        assert_eq!(second.turn_index, 2);
        assert!(!second.output.text.trim().is_empty());
    }

    #[test]
    fn explicit_close_releases_only_the_finished_conversation() {
        let runtime = ConcurrentConversationRuntime::new(2, 8).unwrap();
        runtime
            .process_conversation_turn(&request(1, 1, "회의 시간이 오후 4시야."))
            .unwrap();
        runtime
            .process_conversation_turn(&request(2, 1, "회의 시간이 오후 5시야."))
            .unwrap();
        assert_eq!(runtime.stats().active_conversations, 2);
        assert!(runtime.close_conversation("CONCURRENT-1").unwrap());
        assert_eq!(runtime.stats().active_conversations, 1);
        assert!(!runtime.close_conversation("CONCURRENT-1").unwrap());
        assert!(runtime
            .process_conversation_turn(&request(1, 2, "몇 시라고 했지?"))
            .is_err());
        assert!(runtime
            .process_conversation_turn(&request(2, 2, "몇 시라고 했지?"))
            .is_ok());
        assert!(runtime
            .process_conversation_turn(&request(1, 1, "회의 시간이 오후 6시야."))
            .is_ok());
        assert_eq!(runtime.stats().active_conversations, 2);
    }

    #[test]
    fn profiled_conversation_turn_preserves_response_contract() {
        let runtime = ConcurrentConversationRuntime::new(2, 8).unwrap();
        let request = request(1, 1, "회의 시간이 오후 4시야.");
        let (response, timing) = runtime
            .process_conversation_turn_profiled(&request)
            .unwrap();
        assert!(response.validate_against(&request));
        assert!(
            timing.stages.len() >= 28,
            "profile evidence must retain the authoritative stage trace"
        );

        assert_eq!(runtime.stats().accepted_requests, 1);
    }

    #[test]
    fn invalid_capacity_fails_before_allocating_shards() {
        assert!(matches!(
            ConcurrentConversationRuntime::new(0, 200),
            Err(ConcurrentRuntimeError::InvalidConfiguration)
        ));
        assert!(matches!(
            ConcurrentConversationRuntime::new(8, 4),
            Err(ConcurrentRuntimeError::InvalidConfiguration)
        ));
    }
}
