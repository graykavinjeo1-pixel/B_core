use std::{
    collections::BTreeMap,
    env,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

use semantic_core_adapters::{
    ConcurrentConversationRuntime, ConversationInputModalityIR, ConversationTurnRequestIR,
    ConversationTurnStageTimingIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
    DEFAULT_CONVERSATION_SHARDS,
};
use serde::Serialize;

// Normal operating evidence targets a bounded warm lane.  The separate
// two-hundred-session service test remains the overload boundary canary.
const DEFAULT_CANARY_SESSIONS: usize = 64;
const DEFAULT_CANARY_SHARDS: usize = DEFAULT_CONVERSATION_SHARDS;
const MAX_IN_FLIGHT: usize = 256;
const DEFAULT_CANARY_TEXT: &str = "회의 시간이 오후 4시야.";
const MAX_CANARY_INPUT_BYTES: usize = 16 * 1024;

#[derive(Serialize)]
struct CanaryReport {
    schema: &'static str,
    status: &'static str,
    sessions_started_together: usize,
    input_bytes: usize,
    follow_up_input_bytes: Option<usize>,
    turns_per_session: usize,
    completed: usize,
    failed: usize,
    completed_turns: usize,
    failed_turns: usize,
    shards: usize,
    max_in_flight: usize,
    observed_peak_in_flight: usize,
    rejected_busy: usize,
    wall_ms: f64,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    latency_max_ms: f64,
    latency_p95_budget_ms: Option<f64>,
    latency_p95_budget_passed: bool,
    throughput_requests_per_second: f64,
    stage_latency: Vec<StageLatencyReport>,
    gpu_used: bool,
}

#[derive(Serialize)]
struct StageLatencyReport {
    stage: String,
    samples: usize,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    latency_max_ms: f64,
}

fn request(session: usize, turn_index: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.into(),
        conversation_id: format!("LOAD-SESSION-{session:03}"),
        turn_index,
        request_id: format!("LOAD-REQUEST-{session:03}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.into(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(LanguageCodeIR::Korean),
        context_tags: Vec::new(),
        max_plan_steps: 12,
    }
}

fn main() {
    let sessions = positive_usize_env("B_CORE_CANARY_SESSIONS", DEFAULT_CANARY_SESSIONS);
    let shards = positive_usize_env("B_CORE_CANARY_SHARDS", DEFAULT_CANARY_SHARDS);
    let max_in_flight = positive_usize_env("B_CORE_CANARY_MAX_IN_FLIGHT", MAX_IN_FLIGHT);
    let latency_p95_budget_ms = positive_f64_env("B_CORE_CANARY_MAX_P95_MS");
    let text = Arc::<str>::from(canary_text());
    let follow_up_text = canary_follow_up_text().map(Arc::<str>::from);
    let runtime = Arc::new(
        ConcurrentConversationRuntime::new(shards, max_in_flight)
            .expect("standalone concurrent runtime"),
    );
    let barrier = Arc::new(Barrier::new(sessions + 1));
    let follow_up_barrier = follow_up_text
        .as_ref()
        .map(|_| Arc::new(Barrier::new(sessions + 1)));
    let handles = (0..sessions)
        .map(|session| {
            let runtime = Arc::clone(&runtime);
            let barrier = Arc::clone(&barrier);
            let text = Arc::clone(&text);
            let follow_up_text = follow_up_text.as_ref().map(Arc::clone);
            let follow_up_barrier = follow_up_barrier.as_ref().map(Arc::clone);
            thread::spawn(move || {
                barrier.wait();
                let mut results = Vec::with_capacity(if follow_up_text.is_some() { 2 } else { 1 });
                let first_started = Instant::now();
                let first_request = request(session, 1, &text);
                let first = runtime.process_conversation_turn_profiled(&first_request);
                let first_ok = first
                    .as_ref()
                    .is_ok_and(|(response, _)| response.validate_against(&first_request));
                results.push(match first {
                    Ok((response, timing)) => (
                        response.validate_against(&first_request),
                        first_started.elapsed().as_secs_f64() * 1_000.0,
                        timing.stages,
                    ),
                    Err(_) => (false, first_started.elapsed().as_secs_f64() * 1_000.0, Vec::new()),
                });
                if let (Some(follow_up_text), Some(follow_up_barrier)) =
                    (follow_up_text, follow_up_barrier)
                {
                    follow_up_barrier.wait();
                    if first_ok {
                        let follow_up_started = Instant::now();
                        let follow_up_request = request(
                            session,
                            2,
                            &follow_up_text,
                        );
                        let follow_up = runtime.process_conversation_turn_profiled(&follow_up_request);
                        results.push(match follow_up {
                            Ok((response, timing)) => (
                                response.validate_against(&follow_up_request),
                                follow_up_started.elapsed().as_secs_f64() * 1_000.0,
                                timing.stages,
                            ),
                            Err(_) => (
                                false,
                                follow_up_started.elapsed().as_secs_f64() * 1_000.0,
                                Vec::new(),
                            ),
                        });
                    } else {
                        results.push((false, 0.0, Vec::new()));
                    }
                }
                results
            })
        })
        .collect::<Vec<_>>();
    let wall_started = Instant::now();
    barrier.wait();
    if let Some(follow_up_barrier) = &follow_up_barrier {
        follow_up_barrier.wait();
    }
    let mut results = handles
        .into_iter()
        .map(|handle| handle.join().expect("load worker"))
        .flatten()
        .collect::<Vec<_>>();
    let wall_seconds = wall_started.elapsed().as_secs_f64();
    let turns_per_session = if follow_up_text.is_some() { 2 } else { 1 };
    let completed_turns = results.iter().filter(|(ok, _, _)| *ok).count();
    let completed = results
        .chunks(turns_per_session)
        .filter(|turns| turns.iter().all(|(ok, _, _)| *ok))
        .count();
    results.sort_by(|left, right| left.1.total_cmp(&right.1));
    let stats = runtime.stats();
    let latency_p50_ms = percentile(&results, 0.50);
    let latency_p95_ms = percentile(&results, 0.95);
    let latency_p95_budget_passed =
        latency_p95_budget_ms.is_none_or(|budget| latency_p95_ms <= budget);
    let report = CanaryReport {
        schema: "B_CORE_CONCURRENT_SESSION_CANARY_5",
        status: if completed == sessions
            && stats.rejected_busy_requests == 0
            && latency_p95_budget_passed
        {
            "PASS"
        } else {
            "FAIL"
        },
        sessions_started_together: sessions,
        input_bytes: text.len(),
        follow_up_input_bytes: follow_up_text.as_ref().map(|text| text.len()),
        turns_per_session,
        completed,
        failed: sessions - completed,
        completed_turns,
        failed_turns: results.len() - completed_turns,
        shards,
        max_in_flight,
        observed_peak_in_flight: stats.peak_in_flight,
        rejected_busy: stats.rejected_busy_requests,
        wall_ms: wall_seconds * 1_000.0,
        latency_p50_ms,
        latency_p95_ms,
        latency_max_ms: results.last().map(|result| result.1).unwrap_or_default(),
        latency_p95_budget_ms,
        latency_p95_budget_passed,
        throughput_requests_per_second: completed_turns as f64 / wall_seconds,
        stage_latency: stage_latency(&results),
        gpu_used: false,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("canary report")
    );
    if let Some(milliseconds) = env::var("B_CORE_CANARY_HOLD_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    {
        thread::sleep(Duration::from_millis(milliseconds.min(10_000)));
    }
    if report.status != "PASS" {
        std::process::exit(1);
    }
}

fn canary_text() -> String {
    env::var("B_CORE_CANARY_TEXT")
        .ok()
        .filter(|text| !text.trim().is_empty() && text.len() <= MAX_CANARY_INPUT_BYTES)
        .unwrap_or_else(|| DEFAULT_CANARY_TEXT.into())
}

fn canary_follow_up_text() -> Option<String> {
    env::var("B_CORE_CANARY_FOLLOW_UP_TEXT")
        .ok()
        .filter(|text| !text.trim().is_empty() && text.len() <= MAX_CANARY_INPUT_BYTES)
}


fn positive_usize_env(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn positive_f64_env(name: &str) -> Option<f64> {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn percentile(results: &[(bool, f64, Vec<ConversationTurnStageTimingIR>)], quantile: f64) -> f64 {
    if results.is_empty() {
        return 0.0;
    }
    let index = ((results.len() - 1) as f64 * quantile).round() as usize;
    results[index].1
}

fn stage_latency(
    results: &[(bool, f64, Vec<ConversationTurnStageTimingIR>)],
) -> Vec<StageLatencyReport> {
    let mut per_stage = BTreeMap::<String, Vec<f64>>::new();
    for (ok, _, stages) in results {
        if *ok {
            for stage in stages {
                per_stage
                    .entry(stage.stage.clone())
                    .or_default()
                    .push(stage.elapsed_micros as f64 / 1_000.0);
            }
        }
    }
    per_stage
        .into_iter()
        .map(|(stage, mut values)| {
            values.sort_by(f64::total_cmp);
            let samples = values.len();
            let percentile = |quantile: f64| {
                let index = ((samples - 1) as f64 * quantile).round() as usize;
                values[index]
            };
            StageLatencyReport {
                stage,
                samples,
                latency_p50_ms: percentile(0.50),
                latency_p95_ms: percentile(0.95),
                latency_max_ms: values[samples - 1],
            }
        })
        .collect()
}
